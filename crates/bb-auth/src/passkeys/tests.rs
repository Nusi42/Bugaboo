//! Tests of the passkeys: registration, sign-in, and passkeys stored by an earlier library version.

use base64ct::{Base64UrlUnpadded, Encoding as _};
use bb_core::id::SequentialIds;
use bb_dev_words::accounts::PUBLIC_URL;
use webauthn_authenticator_rs::prelude::WebauthnAuthenticator as _;
use webauthn_authenticator_rs::softpasskey::SoftPasskey;

use super::*;

fn party() -> RelyingParty {
    RelyingParty::from_public_url(PUBLIC_URL, "Bugaboo").unwrap()
}

fn now() -> Timestamp {
    Timestamp::from_second(bb_dev_words::accounts::CLOCK_START).unwrap()
}

fn person() -> UserId {
    UserId::generate(&mut SequentialIds::default())
}

/// A device that does everything the user would do, without asking.
fn device() -> SoftPasskey {
    SoftPasskey::new(true)
}

fn origin() -> Url {
    Url::parse(PUBLIC_URL).unwrap()
}

fn register(passkeys: &Passkeys, device: &mut SoftPasskey, person: UserId) -> StoredCredential {
    let (id, options) = passkeys
        .start_registration(now(), None, person, "ada@example.com", "Ada", &[])
        .unwrap();
    let options = serde_json::from_value(options).unwrap();
    let answer = device.do_registration(origin(), options).unwrap();
    passkeys
        .finish_registration(now(), id, person, &serde_json::to_value(answer).unwrap())
        .unwrap()
}

#[test]
fn auth_02_a_new_passkey_is_asked_to_be_discoverable_when_the_device_can() {
    let passkeys = Passkeys::new(&party()).unwrap();
    let (_, options) = passkeys
        .start_registration(now(), None, person(), "ada@example.com", "Ada", &[])
        .unwrap();
    let selection = &options["publicKey"]["authenticatorSelection"];
    assert_eq!(selection["residentKey"], "preferred", "{options}");
}

#[test]
fn auth_02_the_relying_party_is_a_host_name_with_a_web_address() {
    let public = Url::parse(PUBLIC_URL).unwrap();
    assert_eq!(party().id(), public.host_str().unwrap());
    assert_eq!(party().origin(), format!("{PUBLIC_URL}/"));
    for bad in [
        "",
        "not a url",
        "https://127.0.0.1:47200",
        "ftp://bugaboo.test",
    ] {
        assert!(
            RelyingParty::from_public_url(bad, "Bugaboo").is_err(),
            "{bad}"
        );
    }
}

#[tokio::test]
async fn auth_02_a_passkey_is_registered_and_used_as_the_second_step() {
    let passkeys = Passkeys::new(&party()).unwrap();
    let mut device = device();
    let person = person();
    let stored = register(&passkeys, &mut device, person);
    assert!(!stored.credential_id.is_empty());

    let (id, options) = passkeys
        .start_authentication(now(), None, person, std::slice::from_ref(&stored))
        .unwrap();
    let answer = device
        .do_authentication(origin(), serde_json::from_value(options).unwrap())
        .unwrap();
    let check = passkeys
        .finish_authentication(
            now(),
            id,
            person,
            &serde_json::to_value(answer).unwrap(),
            std::slice::from_ref(&stored),
        )
        .unwrap();
    assert_eq!(check.credential_id, stored.credential_id);
}

/// The soft device cannot look up its own passkeys, as a phone or a laptop does when it holds
/// a discoverable passkey. This does that part: it names the passkey in the request, and puts
/// the person into the answer, as such a device would.
fn answer_like_a_phone(
    device: &mut SoftPasskey,
    mut options: Value,
    stored: &StoredCredential,
    person: UserId,
) -> Value {
    options["publicKey"]["allowCredentials"] = serde_json::json!([{
        "type": "public-key",
        "id": Base64UrlUnpadded::encode_string(&stored.credential_id),
    }]);
    let answer = device
        .do_authentication(origin(), serde_json::from_value(options).unwrap())
        .unwrap();
    let mut answer = serde_json::to_value(answer).unwrap();
    answer["response"]["userHandle"] =
        Value::String(Base64UrlUnpadded::encode_string(&person.to_bytes()));
    answer
}

#[tokio::test]
async fn auth_02_a_passkey_signs_in_on_its_own_and_names_the_person() {
    let passkeys = Passkeys::new(&party()).unwrap();
    let mut device = device();
    let person = person();
    let stored = register(&passkeys, &mut device, person);

    let (id, options) = passkeys.start_discoverable(now(), None).unwrap();
    assert_eq!(
        options["publicKey"]["allowCredentials"],
        serde_json::json!([])
    );
    let answer = answer_like_a_phone(&mut device, options, &stored, person);
    let found = passkeys.identify(&answer).unwrap();
    assert_eq!(found.person, person);
    assert_eq!(found.credential_id, stored.credential_id);
    let check = passkeys
        .finish_discoverable(now(), id, &answer, std::slice::from_ref(&stored))
        .unwrap();
    assert_eq!(check.credential_id, stored.credential_id);
}

#[tokio::test]
async fn sec_12_a_challenge_works_once_and_only_for_the_person_it_was_made_for() {
    let passkeys = Passkeys::new(&party()).unwrap();
    let mut device = device();
    let person = person();
    let stored = register(&passkeys, &mut device, person);
    let (id, options) = passkeys
        .start_authentication(now(), None, person, std::slice::from_ref(&stored))
        .unwrap();
    let answer = serde_json::to_value(
        device
            .do_authentication(origin(), serde_json::from_value(options).unwrap())
            .unwrap(),
    )
    .unwrap();
    // Another person cannot finish it, and trying uses it up.
    let mut ids = SequentialIds::default();
    let _first = UserId::generate(&mut ids);
    let other = UserId::generate(&mut ids);
    assert_eq!(
        passkeys.finish_authentication(now(), id, other, &answer, std::slice::from_ref(&stored)),
        Err(AuthError::Challenge)
    );
    assert_eq!(
        passkeys.finish_authentication(now(), id, person, &answer, std::slice::from_ref(&stored)),
        Err(AuthError::Challenge)
    );
}

#[tokio::test]
async fn sec_14_one_network_and_one_kind_of_ceremony_cannot_fill_the_waiting_challenges() {
    let passkeys = Passkeys::new(&party()).unwrap();
    let mut device = device();
    let person = person();
    let stored = register(&passkeys, &mut device, person);
    let busy = Some("203.0.113.0");
    // One network can start only so many anonymous sign-ins; another network still can.
    for _ in 0..MAX_PENDING_PER_NETWORK {
        passkeys.start_discoverable(now(), busy).unwrap();
    }
    assert_eq!(
        passkeys.start_discoverable(now(), busy).unwrap_err(),
        AuthError::TooManyChallenges
    );
    passkeys
        .start_discoverable(now(), Some("198.51.100.0"))
        .unwrap();
    // Many networks together fill the pool of anonymous sign-ins ...
    let mut network = 0;
    while passkeys
        .start_discoverable(now(), Some(&format!("net-{network}")))
        .is_ok()
    {
        network += 1;
    }
    assert!(network > 10, "the pool took {network} networks");
    // ... and the second step and adding a passkey, even from the busy network, still start.
    passkeys
        .start_authentication(now(), busy, person, std::slice::from_ref(&stored))
        .unwrap();
    passkeys
        .start_registration(now(), busy, person, "ada@example.com", "Ada", &[])
        .unwrap();
    // Challenges that ran out make room again.
    let later = Timestamp::from_second(now().as_second() + 301).unwrap();
    passkeys.start_discoverable(later, busy).unwrap();
}

#[tokio::test]
async fn sec_12_an_old_challenge_is_refused() {
    let passkeys = Passkeys::new(&party()).unwrap();
    let (id, _) = passkeys.start_discoverable(now(), None).unwrap();
    let later = Timestamp::from_second(now().as_second() + 301).unwrap();
    assert_eq!(
        passkeys.finish_discoverable(later, id, &serde_json::json!({}), &[]),
        Err(AuthError::Challenge)
    );
}

#[tokio::test]
async fn auth_02_an_answer_from_another_address_is_refused() {
    // The server expects pages from port 9999; the device was talking to port 47200.
    let other = RelyingParty::from_public_url("https://bugaboo.test:9999", "Bugaboo").unwrap();
    let passkeys = Passkeys::new(&other).unwrap();
    let mut device = device();
    let person = person();
    let (id, options) = passkeys
        .start_registration(now(), None, person, "ada@example.com", "Ada", &[])
        .unwrap();
    let answer = device
        .do_registration(origin(), serde_json::from_value(options).unwrap())
        .unwrap();
    assert_eq!(
        passkeys.finish_registration(now(), id, person, &serde_json::to_value(answer).unwrap()),
        Err(AuthError::Passkey)
    );
}

#[test]
fn auth_02_text_that_is_not_a_challenge_id_is_refused() {
    let id = ChallengeId([9; 16]);
    assert_eq!(ChallengeId::parse(&id.encode()), Some(id));
    for bad in ["", "short", "!!!!!!!!!!!!!!!!!!!!!!", &"a".repeat(30)] {
        assert_eq!(ChallengeId::parse(bad), None, "{bad}");
    }
}

/// A passkey as the previous library version (the pre-release `0.6.1-dev`, the only version that
/// ever wrote one) stored it.
const STORED_BEFORE_THE_UPDATE: &str = r#"{"cred":{"cred_id":"aLlxp4NZB0w9O9BK0fcYQBQzR-My5V_V2Em0cWZlosA","cred":{"type_":"ES256","key":{"EC_EC2":{"curve":"SECP256R1","x":"lV9iD6suTzSQ-r1axZ6IJDdwop2HScrWRItXxz7E17w","y":"7LGT2wO8eshCRYbAmDKQ1uBNwR5t6L0W439iqmw4hFQ"}}},"counter":0,"transports":null,"user_verified":true,"backup_eligible":false,"backup_state":false,"registration_policy":"required","extensions":{"cred_protect":"Ignored","hmac_create_secret":"NotRequested","appid":"NotRequested","cred_props":"Ignored"},"attestation":{"data":"Self_","metadata":"None"},"attestation_format":"packed"}}"#;

#[test]
fn auth_02_a_passkey_stored_by_the_previous_library_version_is_still_read() {
    let credential_id =
        Base64UrlUnpadded::decode_vec("aLlxp4NZB0w9O9BK0fcYQBQzR-My5V_V2Em0cWZlosA").unwrap();
    let data = STORED_BEFORE_THE_UPDATE.to_owned();
    let passkey = StoredCredential {
        credential_id,
        data,
    }
    .passkey()
    .unwrap();
    let written = StoredCredential::of(&passkey).unwrap();
    assert_eq!(written.credential_id, passkey.cred_id().as_slice());
    let text = |json: &str| serde_json::from_str::<Value>(json).unwrap();
    assert_eq!(text(&written.data), text(STORED_BEFORE_THE_UPDATE));
}
