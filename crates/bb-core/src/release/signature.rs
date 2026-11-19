//! The signatures of a release (SEC-18, ADR-0047 decision 2): Ed25519 in the minisign format, read
//! with `minisign-verify`, against the list of our public keys.
//!
//! A signature file holds an untrusted comment, the signature of the file, a trusted comment and a
//! global signature over the signature and the trusted comment. Our trusted comment names the
//! file and the version ([`TrustedComment`]), so a file cannot be passed off as another one or as
//! another version without the check failing.

use minisign_verify::{Error, PublicKey, Signature};

/// One public key that releases are signed with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReleaseKey {
    /// What the key is for, as the release guide names it: `release` or `backup`.
    pub name: &'static str,
    /// The key in the minisign format: base64 of the algorithm, the key ID and the Ed25519 key, as
    /// the second line of a `.pub` file of minisign.
    pub public: &'static str,
}

/// The keys a file of a release may be signed with: the release key and the backup key that is
/// kept apart (plan B). When one is lost or leaked, the next release is signed with the other and
/// drops the bad one from this list (the step *Replace a key* of the release guide). Copied into
/// `SECURITY.md` and the release notes by `cargo xtask gen release` and `cargo xtask release`.
///
/// **These are test keys** (see [`KEYS_ARE_FOR_TESTS`]): their secret keys were thrown away when
/// they were made. The real keys are made with `cargo xtask release keygen`, kept offline, and
/// their public keys replace these before the first release that people install.
pub const RELEASE_PUBLIC_KEYS: &[ReleaseKey] = &[
    ReleaseKey {
        name: "release",
        public: "RWRZmPMPzOsspbnI/J7vVDOJ1q3qHZ+oPwM23I6LcqYtReRagW+IsYe/",
    },
    ReleaseKey {
        name: "backup",
        public: "RWRK0RL2nc9pmFKXiIWQtp+OIHiivdKULp6A7zjkGeiA/c8MMkyBZ5XD",
    },
];

/// Whether [`RELEASE_PUBLIC_KEYS`] are test keys, which nobody can sign with. The release notes
/// and `SECURITY.md` say so while it is `true`; it becomes `false` in the change that puts the real
/// keys in.
pub const KEYS_ARE_FOR_TESTS: bool = true;

/// The name of the list of checksums of a release, beside its files.
pub const CHECKSUMS_FILE: &str = "SHA256SUMS";

/// The ending of the signature of a file: `bugaboo-0.6.0-linux-x64.minisig`.
const SIGNATURE_END: &str = ".minisig";

/// The name of the signature of the file `name`: `bugaboo-0.6.0-linux-x64.minisig`.
#[must_use]
pub fn signature_name(name: &str) -> String {
    format!("{name}{SIGNATURE_END}")
}

/// What the trusted comment of our signatures says: the name of the file and the version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrustedComment {
    /// The name of the file as the release names it.
    pub file: String,
    /// The version of the release.
    pub version: String,
}

impl TrustedComment {
    const FILE: &'static str = "file:";
    const VERSION: &'static str = "version:";

    /// The text of the comment, as the signer writes it: `file:<name>` and `version:<version>`,
    /// separated by a tab like the comments of the minisign tool.
    #[must_use]
    pub fn text(&self) -> String {
        format!(
            "{}{}\t{}{}",
            Self::FILE,
            self.file,
            Self::VERSION,
            self.version
        )
    }

    /// Reads a comment; `None` when it does not name both a file and a version.
    #[must_use]
    pub fn read(text: &str) -> Option<Self> {
        let mut file = None;
        let mut version = None;
        for part in text.split('\t') {
            if let Some(name) = part.strip_prefix(Self::FILE) {
                file = Some(name.to_owned());
            } else if let Some(number) = part.strip_prefix(Self::VERSION) {
                version = Some(number.to_owned());
            }
        }
        Some(Self {
            file: file?,
            version: version?,
        })
    }
}

/// Why a signature does not hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignatureProblem {
    /// The signature file is not one in the minisign format we sign with.
    Unreadable,
    /// The file was signed with a key that is not one of ours.
    UnknownKey,
    /// The signature is from one of our keys, but the file or the trusted comment was changed.
    Changed,
}

/// Checks that `signature` (the text of a `.minisig` file) signs `bytes` with one of `keys`, and
/// gives back its trusted comment. Only signatures of the whole file's hash (the minisign default
/// since 0.8) are taken; the old kind is refused as unreadable.
///
/// # Errors
/// The [`SignatureProblem`] that stops it.
pub fn verify(
    bytes: &[u8],
    signature: &str,
    keys: &[ReleaseKey],
) -> Result<String, SignatureProblem> {
    let signature = Signature::decode(signature).map_err(|_| SignatureProblem::Unreadable)?;
    let mut problem = SignatureProblem::UnknownKey;
    for key in keys {
        let Ok(public) = PublicKey::from_base64(key.public) else {
            continue;
        };
        match public.verify(bytes, &signature, false) {
            Ok(()) => return Ok(signature.trusted_comment().to_owned()),
            Err(Error::UnexpectedKeyId) => {}
            Err(Error::InvalidSignature) => problem = SignatureProblem::Changed,
            Err(_) => return Err(SignatureProblem::Unreadable),
        }
    }
    Err(problem)
}

/// The checksum that the list `sums` (the text of `SHA256SUMS`, one `<hex>  <name>` a line, as
/// `sha256sum` writes it) gives the file `name`, in lower case.
#[must_use]
pub fn checksum_of(sums: &str, name: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let (sum, rest) = line.split_once(' ')?;
        // `sha256sum` marks a file read as binary with `*` before its name.
        let listed = rest.trim_start_matches(' ').trim_start_matches('*');
        (listed == name && sum.len() == 64 && sum.bytes().all(|byte| byte.is_ascii_hexdigit()))
            .then(|| sum.to_ascii_lowercase())
    })
}

/// One line of `SHA256SUMS` for the file `name` with the checksum `sum`.
#[must_use]
pub fn checksum_line(sum: &str, name: &str) -> String {
    format!("{sum}  {name}\n")
}
