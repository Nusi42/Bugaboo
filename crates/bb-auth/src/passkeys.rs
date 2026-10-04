//! Passkeys (AUTH-02): registration, and signing in with a passkey as the second step or on its
//! own. A thin layer over `webauthn-rs` that speaks JSON and the types of `bb-core`.
//!
//! A passkey ceremony has two halves: the server makes a challenge, the browser asks the device,
//! the server checks the answer. The state between the halves lives here, **in memory and for one
//! use**: an entry is removed when it is looked at (also when the answer is wrong), expires after
//! [`CHALLENGE_LIFETIME`], and the table is small (a restart only means a person tries again).
//! Nothing in it is personal data worth keeping.
//!
//! The relying party is the server's public address from the start-up settings. Passkeys are tied
//! to its host name, so changing the host name makes existing passkeys stop working; the
//! settings docs say so.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use base64ct::{Base64UrlUnpadded, Encoding as _};
use bb_core::id::UserId;
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use webauthn_rs::prelude::{
    CredentialID, DiscoverableAuthentication, DiscoverableKey, Passkey, PasskeyAuthentication,
    PasskeyRegistration, PublicKeyCredential, RegisterPublicKeyCredential, Url, Uuid, Webauthn,
    WebauthnBuilder,
};

use crate::error::{AuthError, random};

/// How long a person has to answer a challenge.
pub const CHALLENGE_LIFETIME: Duration = Duration::from_mins(5);
/// How many challenges wait at the same time.
const MAX_PENDING: usize = 1000;

/// The address people reach the server at (`public_url`), cut down to its origin
/// (`scheme://host[:port]`). Only `http` and `https` addresses with a host, and without a path
/// beyond `/`, a query, a fragment or a user name, are accepted.
#[must_use]
pub fn public_origin(text: &str) -> Option<String> {
    let url = Url::parse(text.trim()).ok()?;
    let plain = matches!(url.scheme(), "http" | "https")
        && url.host_str().is_some()
        && matches!(url.path(), "" | "/")
        && url.query().is_none()
        && url.fragment().is_none()
        && url.username().is_empty()
        && url.password().is_none();
    plain.then(|| url.origin().ascii_serialization())
}

/// The server as passkeys know it: the host name they are tied to and the address pages are
/// served from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelyingParty {
    id: String,
    origin: Url,
    name: String,
}

impl RelyingParty {
    /// The relying party for a public address like `https://bugaboo.example.com:47200`. Its host
    /// name becomes the ID of the relying party; browsers accept passkeys only for names, never for
    /// IP addresses. `name` is what the device shows ("Bugaboo").
    ///
    /// # Errors
    /// Fails when the address is not a web address with a host name.
    pub fn from_public_url(url: &str, name: &str) -> Result<Self, AuthError> {
        let origin = Url::parse(url).map_err(|_| AuthError::RelyingParty)?;
        let id = origin.domain().ok_or(AuthError::RelyingParty)?.to_owned();
        if !matches!(origin.scheme(), "http" | "https") {
            return Err(AuthError::RelyingParty);
        }
        Ok(Self {
            id,
            origin,
            name: name.to_owned(),
        })
    }

    /// The host name passkeys are tied to.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The address passkeys are accepted from.
    #[must_use]
    pub fn origin(&self) -> &str {
        self.origin.as_str()
    }
}

/// Names one waiting challenge. The browser app hands it back with the answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChallengeId([u8; 16]);

impl ChallengeId {
    /// The ID as text.
    #[must_use]
    pub fn encode(&self) -> String {
        Base64UrlUnpadded::encode_string(&self.0)
    }

    /// The ID written by [`Self::encode`].
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let mut bytes = [0_u8; 16];
        let decoded = Base64UrlUnpadded::decode(text, &mut bytes).ok()?;
        (decoded.len() == 16).then_some(Self(bytes))
    }
}

/// A passkey as it is stored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredCredential {
    /// The ID the device gave the passkey.
    pub credential_id: Vec<u8>,
    /// The public key and the counter, as JSON.
    pub data: String,
}

impl StoredCredential {
    fn of(passkey: &Passkey) -> Result<Self, AuthError> {
        Ok(Self {
            credential_id: passkey.cred_id().clone(),
            data: serde_json::to_string(passkey).map_err(|_| AuthError::Passkey)?,
        })
    }

    fn passkey(&self) -> Result<Passkey, AuthError> {
        serde_json::from_str(&self.data).map_err(|_| AuthError::Passkey)
    }
}

/// Who a passkey answer says it is from, before it is checked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoverableMatch {
    /// The person the device says the passkey belongs to.
    pub person: UserId,
    /// The ID of the passkey the device used.
    pub credential_id: Vec<u8>,
}

/// A passkey answer that checked out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PasskeyCheck {
    /// The ID of the passkey that was used.
    pub credential_id: Vec<u8>,
    /// The passkey as it must be stored from now on, when its counter or flags changed.
    pub updated: Option<StoredCredential>,
}

enum Ceremony {
    Register(PasskeyRegistration),
    Authenticate(PasskeyAuthentication),
    Discoverable(DiscoverableAuthentication),
}

struct Pending {
    expires: i64,
    person: Option<UserId>,
    ceremony: Ceremony,
}

/// Starts and finishes passkey ceremonies.
pub struct Passkeys {
    webauthn: Webauthn,
    pending: Mutex<HashMap<ChallengeId, Pending>>,
}

impl std::fmt::Debug for Passkeys {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("Passkeys").finish_non_exhaustive()
    }
}

fn uuid_of(person: UserId) -> Uuid {
    Uuid::from_bytes(person.to_bytes())
}

fn json<T: Serialize>(value: &T) -> Result<Value, AuthError> {
    serde_json::to_value(value).map_err(|_| AuthError::Passkey)
}

fn from_json<T: for<'de> Deserialize<'de>>(value: &Value) -> Result<T, AuthError> {
    T::deserialize(value).map_err(|_| AuthError::Passkey)
}

fn credentials(stored: &[StoredCredential]) -> Result<Vec<Passkey>, AuthError> {
    stored.iter().map(StoredCredential::passkey).collect()
}

impl Passkeys {
    /// Passkeys for this relying party.
    ///
    /// # Errors
    /// Fails when the settings are not usable.
    pub fn new(party: &RelyingParty) -> Result<Self, AuthError> {
        let webauthn = WebauthnBuilder::new(&party.id, &party.origin)
            .map_err(|_| AuthError::RelyingParty)?
            .rp_name(&party.name)
            .timeout(CHALLENGE_LIFETIME)
            .build()
            .map_err(|_| AuthError::RelyingParty)?;
        Ok(Self {
            webauthn,
            pending: Mutex::new(HashMap::new()),
        })
    }

    fn put(
        &self,
        now: Timestamp,
        person: Option<UserId>,
        ceremony: Ceremony,
    ) -> Result<ChallengeId, AuthError> {
        let id = ChallengeId(random()?);
        let lifetime = i64::try_from(CHALLENGE_LIFETIME.as_secs()).unwrap_or(i64::MAX);
        let mut pending = self.pending.lock().unwrap_or_else(PoisonError::into_inner);
        if pending.len() >= MAX_PENDING {
            pending.retain(|_, entry| entry.expires > now.as_second());
            if pending.len() >= MAX_PENDING {
                return Err(AuthError::TooManyChallenges);
            }
        }
        pending.insert(
            id,
            Pending {
                expires: now.as_second().saturating_add(lifetime),
                person,
                ceremony,
            },
        );
        Ok(id)
    }

    /// Takes a challenge out of the table: it is used up whether or not the answer is right.
    fn take(
        &self,
        now: Timestamp,
        id: ChallengeId,
        person: Option<UserId>,
    ) -> Result<Ceremony, AuthError> {
        let entry = self
            .pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&id)
            .ok_or(AuthError::Challenge)?;
        if entry.expires <= now.as_second() || entry.person != person {
            return Err(AuthError::Challenge);
        }
        Ok(entry.ceremony)
    }

    /// Starts adding a passkey for `person`. `name` and `display_name` are what the device shows;
    /// passkeys the person has already are excluded, so one device is not added twice.
    ///
    /// # Errors
    /// Fails when the challenge cannot be made.
    pub fn start_registration(
        &self,
        now: Timestamp,
        person: UserId,
        name: &str,
        display_name: &str,
        existing: &[StoredCredential],
    ) -> Result<(ChallengeId, Value), AuthError> {
        let exclude: Vec<CredentialID> = credentials(existing)?
            .iter()
            .map(|passkey| passkey.cred_id().clone())
            .collect();
        let (options, state) = self
            .webauthn
            .start_passkey_registration(
                uuid_of(person),
                name,
                display_name,
                (!exclude.is_empty()).then_some(exclude),
            )
            .map_err(|_| AuthError::Passkey)?;
        let id = self.put(now, Some(person), Ceremony::Register(state))?;
        Ok((id, json(&options)?))
    }

    /// Finishes adding a passkey.
    ///
    /// # Errors
    /// Fails when the challenge is unknown, used, too old or for another person, or when the
    /// answer of the device does not check out.
    pub fn finish_registration(
        &self,
        now: Timestamp,
        id: ChallengeId,
        person: UserId,
        answer: &Value,
    ) -> Result<StoredCredential, AuthError> {
        let Ceremony::Register(state) = self.take(now, id, Some(person))? else {
            return Err(AuthError::Challenge);
        };
        let answer: RegisterPublicKeyCredential = from_json(answer)?;
        let passkey = self
            .webauthn
            .finish_passkey_registration(&answer, &state)
            .map_err(|_| AuthError::Passkey)?;
        StoredCredential::of(&passkey)
    }

    /// Starts checking one of the passkeys of `person`.
    ///
    /// # Errors
    /// Fails when the person has no usable passkey or the challenge cannot be made.
    pub fn start_authentication(
        &self,
        now: Timestamp,
        person: UserId,
        stored: &[StoredCredential],
    ) -> Result<(ChallengeId, Value), AuthError> {
        let (options, state) = self
            .webauthn
            .start_passkey_authentication(&credentials(stored)?)
            .map_err(|_| AuthError::Passkey)?;
        let id = self.put(now, Some(person), Ceremony::Authenticate(state))?;
        Ok((id, json(&options)?))
    }

    /// Finishes checking a passkey of `person`.
    ///
    /// # Errors
    /// Fails when the challenge is unknown, used, too old or for another person, or when the
    /// answer does not check out.
    pub fn finish_authentication(
        &self,
        now: Timestamp,
        id: ChallengeId,
        person: UserId,
        answer: &Value,
        stored: &[StoredCredential],
    ) -> Result<PasskeyCheck, AuthError> {
        let Ceremony::Authenticate(state) = self.take(now, id, Some(person))? else {
            return Err(AuthError::Challenge);
        };
        let answer: PublicKeyCredential = from_json(answer)?;
        let result = self
            .webauthn
            .finish_passkey_authentication(&answer, &state)
            .map_err(|_| AuthError::Passkey)?;
        Self::checked(&result, stored)
    }

    /// Starts signing in with a passkey and nothing else: the device offers the passkeys it holds
    /// for this server, so nobody has to type an address first (and nothing tells whether an
    /// address has an account).
    ///
    /// # Errors
    /// Fails when the challenge cannot be made.
    pub fn start_discoverable(&self, now: Timestamp) -> Result<(ChallengeId, Value), AuthError> {
        let (options, state) = self
            .webauthn
            .start_discoverable_authentication()
            .map_err(|_| AuthError::Passkey)?;
        let id = self.put(now, None, Ceremony::Discoverable(state))?;
        Ok((id, json(&options)?))
    }

    /// Reads from the answer which person and which passkey the device used. This proves
    /// nothing yet; [`Self::finish_discoverable`] checks the answer against the stored passkey.
    ///
    /// # Errors
    /// Fails when the answer does not say who it is from.
    pub fn identify(&self, answer: &Value) -> Result<DiscoverableMatch, AuthError> {
        let answer: PublicKeyCredential = from_json(answer)?;
        let (uuid, credential_id) = self
            .webauthn
            .identify_discoverable_authentication(&answer)
            .map_err(|_| AuthError::Passkey)?;
        let person = UserId::from_bytes(*uuid.as_bytes()).map_err(|_| AuthError::Passkey)?;
        Ok(DiscoverableMatch {
            person,
            credential_id: credential_id.to_vec(),
        })
    }

    /// Finishes signing in with a passkey alone. `stored` are the passkeys of the person that
    /// [`Self::identify`] named.
    ///
    /// # Errors
    /// Fails when the challenge is unknown, used or too old, or when the answer does not check
    /// out.
    pub fn finish_discoverable(
        &self,
        now: Timestamp,
        id: ChallengeId,
        answer: &Value,
        stored: &[StoredCredential],
    ) -> Result<PasskeyCheck, AuthError> {
        let Ceremony::Discoverable(state) = self.take(now, id, None)? else {
            return Err(AuthError::Challenge);
        };
        let answer: PublicKeyCredential = from_json(answer)?;
        let keys: Vec<DiscoverableKey> = credentials(stored)?
            .iter()
            .map(DiscoverableKey::from)
            .collect();
        let result = self
            .webauthn
            .finish_discoverable_authentication(&answer, state, &keys)
            .map_err(|_| AuthError::Passkey)?;
        Self::checked(&result, stored)
    }

    fn checked(
        result: &webauthn_rs::prelude::AuthenticationResult,
        stored: &[StoredCredential],
    ) -> Result<PasskeyCheck, AuthError> {
        let mut updated = None;
        for credential in stored {
            let mut passkey = credential.passkey()?;
            if passkey.update_credential(result) == Some(true) {
                updated = Some(StoredCredential::of(&passkey)?);
            }
        }
        Ok(PasskeyCheck {
            credential_id: result.cred_id().clone(),
            updated,
        })
    }
}

#[cfg(test)]
mod origin_tests {
    use super::public_origin;

    #[test]
    fn auth_02_the_public_address_is_cut_down_to_its_origin() {
        assert_eq!(
            public_origin(" https://Bugaboo.Example.com:47200/ ").as_deref(),
            Some("https://bugaboo.example.com:47200")
        );
        assert_eq!(
            public_origin("http://localhost").as_deref(),
            Some("http://localhost")
        );
        for bad in [
            "",
            "bugaboo.example.com",
            "ftp://example.com",
            "https://example.com/app",
            "https://example.com/?x=1",
            "https://example.com/#top",
            "https://user:pass@example.com",
            "https://",
        ] {
            assert_eq!(public_origin(bad), None, "{bad}");
        }
    }
}

#[cfg(test)]
mod tests {
    use base64ct::{Base64UrlUnpadded, Encoding as _};
    use bb_core::id::SequentialIds;
    use webauthn_authenticator_rs::prelude::WebauthnAuthenticator as _;
    use webauthn_authenticator_rs::softpasskey::SoftPasskey;

    use super::*;

    fn party() -> RelyingParty {
        RelyingParty::from_public_url("https://bugaboo.test:47200", "Bugaboo").unwrap()
    }

    fn now() -> Timestamp {
        Timestamp::from_second(1_800_000_000).unwrap()
    }

    fn person() -> UserId {
        UserId::generate(&mut SequentialIds::default())
    }

    /// A device that does everything the user would do, without asking.
    fn device() -> SoftPasskey {
        SoftPasskey::new(true)
    }

    fn origin() -> Url {
        Url::parse("https://bugaboo.test:47200").unwrap()
    }

    fn register(passkeys: &Passkeys, device: &mut SoftPasskey, person: UserId) -> StoredCredential {
        let (id, options) = passkeys
            .start_registration(now(), person, "ada@example.com", "Ada", &[])
            .unwrap();
        let options = serde_json::from_value(options).unwrap();
        let answer = device.do_registration(origin(), options).unwrap();
        passkeys
            .finish_registration(now(), id, person, &serde_json::to_value(answer).unwrap())
            .unwrap()
    }

    #[test]
    fn auth_02_the_relying_party_is_a_host_name_with_a_web_address() {
        assert_eq!(party().id(), "bugaboo.test");
        assert_eq!(party().origin(), "https://bugaboo.test:47200/");
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
            .start_authentication(now(), person, std::slice::from_ref(&stored))
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

        let (id, options) = passkeys.start_discoverable(now()).unwrap();
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
            .start_authentication(now(), person, std::slice::from_ref(&stored))
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
            passkeys.finish_authentication(
                now(),
                id,
                other,
                &answer,
                std::slice::from_ref(&stored)
            ),
            Err(AuthError::Challenge)
        );
        assert_eq!(
            passkeys.finish_authentication(
                now(),
                id,
                person,
                &answer,
                std::slice::from_ref(&stored)
            ),
            Err(AuthError::Challenge)
        );
    }

    #[tokio::test]
    async fn sec_12_an_old_challenge_is_refused() {
        let passkeys = Passkeys::new(&party()).unwrap();
        let (id, _) = passkeys.start_discoverable(now()).unwrap();
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
            .start_registration(now(), person, "ada@example.com", "Ada", &[])
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
}
