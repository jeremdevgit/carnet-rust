//! Joplin end-to-end encryption (E2EE), compatible with Joplin 3.x.
//!
//! Ported from `@joplin/lib/services/e2ee/EncryptionService.ts`; no custom cryptography:
//! - master keys: KeyV1 (AES-256-GCM, PBKDF2-SHA512 220 000 iterations) and legacy SJCL4/SJCL2;
//! - items: StringV1 (AES-256-GCM over UTF-16LE, 64 Ki code units per block) and legacy SJCL CCM;
//! - attachments: FileV1 (AES-256-GCM, 128 KiB blocks) and legacy SJCL CCM.
//!
//! New data is always written with KeyV1/StringV1/FileV1. The legacy OCB2 modes (methods 1 and 2)
//! are deprecated by Joplin as insecure and are refused.

use crate::{Error, Result};
use aes_gcm::{Aes256Gcm, KeyInit, Nonce, aead::Aead};
use base64::{Engine, engine::general_purpose::STANDARD as B64};
use serde_json::{Value, json};
use sha2::{Sha256, Sha512};
use std::collections::HashMap;
use zeroize::Zeroizing;

pub const SJCL3: u8 = 3;
pub const SJCL4: u8 = 4;
pub const SJCL1A: u8 = 5;
pub const SJCL1B: u8 = 7;
pub const KEY_V1: u8 = 8;
pub const FILE_V1: u8 = 9;
pub const STRING_V1: u8 = 10;

const STRING_CHUNK: usize = 65_536; // UTF-16 code units
const FILE_CHUNK: usize = 131_072; // bytes
const MAX_BLOCK: usize = 0xFF_FFFF;
const MAX_PLAIN: usize = 512 * 1024 * 1024;

fn bad(what: &'static str) -> Error {
    Error::Invalid(what)
}

fn random<const N: usize>() -> Result<[u8; N]> {
    let mut b = [0u8; N];
    getrandom::fill(&mut b).map_err(|_| bad("system random generator"))?;
    Ok(b)
}

fn derive512(password: &[u8], salt: &[u8], iterations: u32) -> Zeroizing<[u8; 32]> {
    let mut key = Zeroizing::new([0u8; 32]);
    pbkdf2::pbkdf2_hmac::<Sha512>(password, salt, iterations, key.as_mut());
    key
}

fn derive256(password: &[u8], salt: &[u8], iterations: u32) -> Zeroizing<[u8; 32]> {
    let mut key = Zeroizing::new([0u8; 32]);
    pbkdf2::pbkdf2_hmac::<Sha256>(password, salt, iterations, key.as_mut());
    key
}

/// Joplin's `{salt, iv, ct}` AES-256-GCM envelope with PBKDF2-SHA512.
fn gcm_open(password: &[u8], envelope: &str, iterations: u32) -> Result<Zeroizing<Vec<u8>>> {
    let v: Value = serde_json::from_str(envelope).map_err(|_| bad("encrypted block"))?;
    let field = |k: &str| -> Result<Vec<u8>> {
        B64.decode(v[k].as_str().ok_or(bad("encrypted block"))?)
            .map_err(|_| bad("encrypted block"))
    };
    let (salt, iv, ct) = (field("salt")?, field("iv")?, field("ct")?);
    if iv.len() != 12 || ct.len() < 16 {
        return Err(bad("encrypted block"));
    }
    let key = derive512(password, &salt, iterations);
    let cipher = Aes256Gcm::new_from_slice(key.as_ref()).map_err(|_| bad("key"))?;
    cipher
        .decrypt(Nonce::from_slice(&iv), ct.as_slice())
        .map(Zeroizing::new)
        .map_err(|_| Error::Unlock)
}

fn gcm_seal(password: &[u8], plain: &[u8], iterations: u32) -> Result<String> {
    let salt: [u8; 32] = random()?;
    let iv: [u8; 12] = random()?;
    let key = derive512(password, &salt, iterations);
    let cipher = Aes256Gcm::new_from_slice(key.as_ref()).map_err(|_| bad("key"))?;
    let ct = cipher
        .encrypt(Nonce::from_slice(&iv), plain)
        .map_err(|_| bad("encryption failed"))?;
    Ok(json!({"salt": B64.encode(salt), "iv": B64.encode(iv), "ct": B64.encode(ct)}).to_string())
}

/// SJCL `{iv, iter, ks, ts, mode, salt, ct}` envelope, AES-CCM only.
fn sjcl_open(password: &[u8], envelope: &str) -> Result<Zeroizing<Vec<u8>>> {
    use aes::{Aes128, Aes256};
    use ccm::{
        Ccm,
        consts::{U8, U12, U13},
    };
    let v: Value = serde_json::from_str(envelope).map_err(|_| bad("legacy encrypted block"))?;
    if v["mode"] != "ccm"
        || v["cipher"] != "aes"
        || v["ts"] != 64
        || v["adata"].as_str().is_some_and(|a| !a.is_empty())
    {
        return Err(bad(
            "unsupported legacy Joplin encryption (OCB2 is not supported)",
        ));
    }
    let field = |k: &str| -> Result<Vec<u8>> {
        B64.decode(v[k].as_str().ok_or(bad("legacy encrypted block"))?)
            .map_err(|_| bad("legacy encrypted block"))
    };
    let (iv, salt, ct) = (field("iv")?, field("salt")?, field("ct")?);
    let iter = v["iter"]
        .as_u64()
        .filter(|i| (1..=1_000_000).contains(i))
        .ok_or(bad("legacy iterations"))? as u32;
    let ks = v["ks"].as_u64().ok_or(bad("legacy key size"))?;
    if iv.len() != 16 || ct.len() < 8 {
        return Err(bad("legacy encrypted block"));
    }
    // SJCL shortens the 16-byte IV to 15 - L bytes, L being the size of the length field.
    let plain_len = ct.len() - 8;
    let l = if plain_len >> 16 == 0 { 2 } else { 3 };
    let key = derive256(password, &salt, iter);
    let nonce = &iv[..15 - l];
    let out = match (ks, l) {
        (128, 2) => Ccm::<Aes128, U8, U13>::new_from_slice(&key[..16])
            .ok()
            .and_then(|c| c.decrypt(nonce.into(), ct.as_slice()).ok()),
        (128, 3) => Ccm::<Aes128, U8, U12>::new_from_slice(&key[..16])
            .ok()
            .and_then(|c| c.decrypt(nonce.into(), ct.as_slice()).ok()),
        (256, 2) => Ccm::<Aes256, U8, U13>::new_from_slice(&key[..])
            .ok()
            .and_then(|c| c.decrypt(nonce.into(), ct.as_slice()).ok()),
        (256, 3) => Ccm::<Aes256, U8, U12>::new_from_slice(&key[..])
            .ok()
            .and_then(|c| c.decrypt(nonce.into(), ct.as_slice()).ok()),
        _ => return Err(bad("legacy key size")),
    };
    out.map(Zeroizing::new).ok_or(Error::Unlock)
}

/// JavaScript `unescape()`: `%XX` and `%uXXXX` to UTF-16 code units.
fn js_unescape(s: &str) -> Vec<u16> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let hex = |s: &[u8]| {
        std::str::from_utf8(s)
            .ok()
            .and_then(|h| u16::from_str_radix(h, 16).ok())
    };
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            if b.get(i + 1) == Some(&b'u')
                && let Some(u) = b.get(i + 2..i + 6).and_then(hex)
            {
                out.push(u);
                i += 6;
                continue;
            }
            if let Some(u) = b.get(i + 1..i + 3).and_then(hex) {
                out.push(u);
                i += 3;
                continue;
            }
        }
        let c = s[i..].chars().next().unwrap_or('\u{fffd}');
        let mut buf = [0u16; 2];
        out.extend_from_slice(c.encode_utf16(&mut buf));
        i += c.len_utf8();
    }
    out
}

fn utf16le(units: &[u16]) -> Vec<u8> {
    units.iter().flat_map(|u| u.to_le_bytes()).collect()
}

fn from_utf16le(bytes: &[u8]) -> Result<Vec<u16>> {
    let (pairs, rest) = bytes.as_chunks::<2>();
    if !rest.is_empty() {
        return Err(bad("encrypted text"));
    }
    Ok(pairs.iter().map(|p| u16::from_le_bytes(*p)).collect())
}

/// Decrypted master keys of a sync target. Values are the hexadecimal key material that Joplin
/// uses as the PBKDF2 password for item encryption.
#[derive(Default)]
pub struct Keys {
    keys: HashMap<String, Zeroizing<String>>,
    pub active: String,
}

impl Keys {
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    fn get(&self, id: &str) -> Result<&Zeroizing<String>> {
        self.keys.get(id).ok_or(bad(
            "item encrypted with an unknown master key (shared notebook or other password)",
        ))
    }

    /// Decrypts every master key listed in `info.json` that the password opens; the active key is mandatory.
    pub fn unlock(info: &Value, password: &str) -> Result<Keys> {
        let active = info["activeMasterKeyId"]["value"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let mut keys = HashMap::new();
        for mk in info["masterKeys"].as_array().into_iter().flatten() {
            let id = mk["id"].as_str().unwrap_or_default();
            if !valid_key_id(id) {
                continue;
            }
            if let Ok(k) = decrypt_master_key(mk, password) {
                keys.insert(id.to_string(), k);
            }
        }
        if !keys.contains_key(&active) {
            return Err(Error::Unlock);
        }
        Ok(Keys { keys, active })
    }

    /// Creates a new KeyV1 master key, as Joplin does when encryption is enabled.
    pub fn create(password: &str) -> Result<(Keys, Value)> {
        let material: [u8; 256] = random()?;
        let hexa = Zeroizing::new(hex::encode(material));
        let id = hex::encode(random::<16>()?);
        let now = crate::now();
        let content = gcm_seal(password.as_bytes(), &material, 220_000)?;
        let entity = json!({
            "id": id, "created_time": now, "updated_time": now, "source_application": "net.cozic.joplin-desktop",
            "encryption_method": KEY_V1, "checksum": "", "content": content, "hasBeenUsed": true,
        });
        let mut keys = HashMap::new();
        keys.insert(id.clone(), hexa);
        Ok((Keys { keys, active: id }, entity))
    }
}

pub fn valid_key_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit())
}

fn decrypt_master_key(mk: &Value, password: &str) -> Result<Zeroizing<String>> {
    let content = mk["content"].as_str().ok_or(bad("master key"))?;
    match mk["encryption_method"].as_u64() {
        Some(m) if m == KEY_V1 as u64 => Ok(Zeroizing::new(hex::encode(
            gcm_open(password.as_bytes(), content, 220_000)?.as_slice(),
        ))),
        Some(m) if m == SJCL4 as u64 => {
            let plain = sjcl_open(password.as_bytes(), content)?;
            let s = String::from_utf8(plain.to_vec()).map_err(|_| bad("master key"))?;
            if s.is_empty() || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(Error::Unlock);
            }
            Ok(Zeroizing::new(s))
        }
        _ => Err(bad("unsupported master key encryption method")),
    }
}

pub fn is_encrypted(text: &str) -> bool {
    text.len() >= 5
        && text.starts_with("JED")
        && text.as_bytes()[3..5].iter().all(u8::is_ascii_digit)
}

struct Header<'a> {
    method: u8,
    key_id: &'a str,
    rest: &'a str,
}

fn header(text: &str) -> Result<Header<'_>> {
    if !text.starts_with("JED01") || text.len() < 11 {
        return Err(bad("encrypted data header"));
    }
    let size = usize::from_str_radix(&text[5..11], 16).map_err(|_| bad("encrypted data header"))?;
    if size != 34 || text.len() < 11 + size {
        return Err(bad("encrypted data header"));
    }
    let md = &text[11..11 + size];
    let method = u8::from_str_radix(&md[..2], 16).map_err(|_| bad("encrypted data header"))?;
    let key_id = &md[2..];
    if !valid_key_id(key_id) {
        return Err(bad("encrypted data header"));
    }
    Ok(Header {
        method,
        key_id,
        rest: &text[11 + size..],
    })
}

fn blocks(mut rest: &str) -> Result<Vec<&str>> {
    let mut out = Vec::new();
    while !rest.is_empty() {
        let len = rest
            .get(..6)
            .and_then(|h| usize::from_str_radix(h, 16).ok())
            .ok_or(bad("encrypted block size"))?;
        let block = rest.get(6..6 + len).ok_or(bad("encrypted block size"))?;
        out.push(block);
        rest = &rest[6 + len..];
    }
    Ok(out)
}

fn encode(method: u8, key_id: &str, blocks: &[String]) -> String {
    let mut s = format!("JED01{:06x}{:02x}{key_id}", 2 + key_id.len(), method);
    for b in blocks {
        s.push_str(&format!("{:06x}", b.len()));
        s.push_str(b);
    }
    s
}

/// Decrypts an item's `encryption_cipher_text` into its serialised plain form.
pub fn decrypt_string(keys: &Keys, cipher: &str) -> Result<Zeroizing<String>> {
    let h = header(cipher)?;
    let key = keys.get(h.key_id)?;
    let mut units: Vec<u16> = Vec::new();
    for block in blocks(h.rest)? {
        match h.method {
            STRING_V1 => units.extend(from_utf16le(&gcm_open(key.as_bytes(), block, 3)?)?),
            SJCL1A | SJCL1B | SJCL3 => {
                let plain = sjcl_open(key.as_bytes(), block)?;
                let s = std::str::from_utf8(&plain).map_err(|_| bad("encrypted text"))?;
                if h.method == SJCL3 {
                    units.extend(s.encode_utf16());
                } else {
                    units.extend(js_unescape(s));
                }
            }
            _ => return Err(bad("unsupported Joplin encryption method")),
        }
        if units.len() > MAX_PLAIN {
            return Err(bad("encrypted item too large"));
        }
    }
    String::from_utf16(&units)
        .map(Zeroizing::new)
        .map_err(|_| bad("encrypted text"))
}

/// Encrypts a serialised item with StringV1 and the active master key.
pub fn encrypt_string(keys: &Keys, plain: &str) -> Result<String> {
    let key = keys.get(&keys.active)?;
    let units: Vec<u16> = plain.encode_utf16().collect();
    let blocks = units
        .chunks(STRING_CHUNK)
        .map(|c| gcm_seal(key.as_bytes(), &utf16le(c), 3))
        .collect::<Result<Vec<_>>>()?;
    Ok(encode(STRING_V1, &keys.active, &blocks))
}

/// Decrypts an attachment blob.
pub fn decrypt_file(keys: &Keys, data: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    let text = std::str::from_utf8(data).map_err(|_| bad("encrypted attachment"))?;
    let h = header(text)?;
    let key = keys.get(h.key_id)?;
    let mut out = Zeroizing::new(Vec::new());
    for block in blocks(h.rest)? {
        match h.method {
            FILE_V1 => out.extend_from_slice(&gcm_open(key.as_bytes(), block, 3)?),
            SJCL1A | SJCL1B | SJCL3 => {
                let plain = sjcl_open(key.as_bytes(), block)?;
                let s = std::str::from_utf8(&plain).map_err(|_| bad("encrypted attachment"))?;
                let b64 = if h.method == SJCL3 {
                    s.to_string()
                } else {
                    String::from_utf16(&js_unescape(s)).map_err(|_| bad("encrypted attachment"))?
                };
                out.extend_from_slice(
                    &B64.decode(b64.trim())
                        .map_err(|_| bad("encrypted attachment"))?,
                );
            }
            _ => return Err(bad("unsupported Joplin encryption method")),
        }
        if out.len() > MAX_PLAIN {
            return Err(bad("encrypted attachment too large"));
        }
    }
    Ok(out)
}

/// Encrypts an attachment blob with FileV1 and the active master key.
pub fn encrypt_file(keys: &Keys, data: &[u8]) -> Result<Vec<u8>> {
    let key = keys.get(&keys.active)?;
    let blocks = data
        .chunks(FILE_CHUNK)
        .map(|c| gcm_seal(key.as_bytes(), c, 3))
        .collect::<Result<Vec<_>>>()?;
    if blocks.iter().any(|b| b.len() > MAX_BLOCK) {
        return Err(bad("encrypted block too large"));
    }
    Ok(encode(FILE_V1, &keys.active, &blocks).into_bytes())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Random password so no credential is written in the sources.
    pub(crate) fn random_password() -> String {
        let mut b = [0u8; 16];
        getrandom::fill(&mut b).unwrap();
        hex::encode(b)
    }

    #[test]
    fn roundtrip_strings_files_and_keys() {
        let password = random_password();
        let (keys, entity) = Keys::create(&password).unwrap();
        let info = json!({"activeMasterKeyId": {"value": keys.active}, "masterKeys": [entity]});
        let reopened = Keys::unlock(&info, &password).unwrap();
        assert!(matches!(
            Keys::unlock(&info, &random_password()),
            Err(Error::Unlock)
        ));
        // Long text with a surrogate pair straddling the 64 Ki block boundary.
        let text = format!("{}🙂 é漢", "x".repeat(STRING_CHUNK - 1));
        let c = encrypt_string(&keys, &text).unwrap();
        assert!(is_encrypted(&c));
        assert_eq!(decrypt_string(&reopened, &c).unwrap().as_str(), text);
        let blob: Vec<u8> = (0..300_000u32).map(|i| (i % 251) as u8).collect();
        let f = encrypt_file(&keys, &blob).unwrap();
        assert_eq!(
            decrypt_file(&reopened, &f).unwrap().as_slice(),
            blob.as_slice()
        );
        let mut tampered = c.clone().into_bytes();
        let n = tampered.len() - 5;
        tampered[n] ^= 1;
        assert!(decrypt_string(&reopened, std::str::from_utf8(&tampered).unwrap_or("")).is_err());
    }

    #[test]
    fn unescape_matches_javascript() {
        assert_eq!(
            String::from_utf16(&js_unescape("a%20b%E9%u6F22%uD83D%uDE42")).unwrap(),
            "a bé漢🙂"
        );
    }

    #[test]
    fn rejects_malformed_headers() {
        let keys = Keys::default();
        assert!(decrypt_string(&keys, "JED01").is_err());
        assert!(decrypt_string(&keys, "JED01000022zz").is_err());
        assert!(decrypt_string(&keys, &format!("JED010000220a{}ffffff", "0".repeat(32))).is_err());
    }
}
