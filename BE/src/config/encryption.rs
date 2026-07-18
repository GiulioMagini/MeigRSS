use aes_gcm::{
    aead::{Aead, Generate, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use std::env;

/// Size of AES-256-GCM nonce in bytes.
const NONCE_SIZE: usize = 12;

use std::sync::OnceLock;
use std::path::PathBuf;

/// Singleton encryption key, loaded once and reused for the entire process lifetime.
static ENCRYPTION_KEY: OnceLock<[u8; 32]> = OnceLock::new();

/// Returns the path for the encryption key file, stored next to the database.
fn key_file_path() -> PathBuf {
    let db_path = std::env::var("MEIGRSS_DB_PATH").unwrap_or_else(|_| {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        exe_dir.join("data").join("meigrss.db").to_string_lossy().to_string()
    });
    let data_dir = std::path::Path::new(&db_path)
        .parent()
        .unwrap_or(std::path::Path::new("."));
    data_dir.join(".encryption_key")
}

/// Returns the encryption key. Priority:
/// 1. Environment variable MEIGRSS_ENCRYPTION_KEY (for Docker/advanced users)
/// 2. Key file stored next to the database (auto-generated on first run)
fn get_encryption_key() -> [u8; 32] {
    *ENCRYPTION_KEY.get_or_init(|| {
        // Priority 1: environment variable
        if let Ok(key_b64) = env::var("MEIGRSS_ENCRYPTION_KEY") {
            match BASE64.decode(&key_b64) {
                Ok(key_bytes) if key_bytes.len() == 32 => {
                    let mut key = [0u8; 32];
                    key.copy_from_slice(&key_bytes);
                    return key;
                }
                Ok(key_bytes) => {
                    // Exit with a clear error instead of panicking or silently
                    // falling back to another key (which would make previously
                    // encrypted data undecipherable in a confusing way)
                    tracing::error!(
                        "MEIGRSS_ENCRYPTION_KEY must decode to exactly 32 bytes (got {})",
                        key_bytes.len()
                    );
                    std::process::exit(1);
                }
                Err(e) => {
                    tracing::error!("MEIGRSS_ENCRYPTION_KEY is not valid base64: {}", e);
                    std::process::exit(1);
                }
            }
        }

        // Priority 2: key file
        let path = key_file_path();
        if let Ok(contents) = std::fs::read_to_string(&path) {
            if let Ok(key_bytes) = BASE64.decode(contents.trim()) {
                if key_bytes.len() == 32 {
                    let mut key = [0u8; 32];
                    key.copy_from_slice(&key_bytes);
                    if crate::is_system_italian() {
                        tracing::info!("Chiave di crittografia caricata da {}", path.display());
                    } else {
                        tracing::info!("Encryption key loaded from {}", path.display());
                    }
                    return key;
                }
            }
        }

        // Generate 32 random bytes via the Generate trait (system CSPRNG).
        // An OS RNG failure is unrecoverable: without randomness we cannot
        // create a secure key, so aborting with a clear message is correct.
        let key = <[u8; 32]>::try_generate()
            .expect("OS random number generator failure while generating encryption key");

        // Ensure the directory exists
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        if let Err(e) = std::fs::write(&path, BASE64.encode(&key)) {
            tracing::warn!(
                "Failed to save encryption key to {}: {}. \
                 Key will not persist across restarts.",
                path.display(), e
            );
        } else {
            
            // Restrict key file permissions to the owner only (Unix).
            // On Windows this is a no-op: files under the user profile
            // are already protected by per-user NTFS ACLs.
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
            }

            if crate::is_system_italian() {
                tracing::info!("Chiave di crittografia generata e salvata in {}", path.display());
            } else {
                tracing::info!("Encryption key generated and saved to {}", path.display());
            }
        }

        key
    })
}

/// Encrypts a plaintext API key using AES-256-GCM.
/// Returns (encrypted_data_base64, nonce_base64).
pub fn encrypt_api_key(plaintext: &str) -> Result<(String, String), String> {
    let key = get_encryption_key();
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| format!("Failed to create cipher: {}", e))?;

    // Generate random nonce
    // Generate a random nonce via the Generate trait (system CSPRNG)
    let nonce_bytes = <[u8; NONCE_SIZE]>::try_generate()
        .map_err(|e| format!("OS RNG failure while generating nonce: {}", e))?;
    // From<[u8; N]> replaces the deprecated Nonce::from_slice
    let nonce = Nonce::from(nonce_bytes);

    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| format!("Encryption failed: {}", e))?;

    Ok((
        BASE64.encode(&ciphertext),
        BASE64.encode(&nonce_bytes),
    ))
}

/// Decrypts an API key from its stored encrypted form.
/// Takes the encrypted_data_base64 and nonce_base64 from the database.
pub fn decrypt_api_key(encrypted_b64: &str, nonce_b64: &str) -> Result<String, String> {
    let key = get_encryption_key();
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| format!("Failed to create cipher: {}", e))?;

    let ciphertext = BASE64.decode(encrypted_b64)
        .map_err(|e| format!("Invalid base64 ciphertext: {}", e))?;
    let nonce_bytes = BASE64.decode(nonce_b64)
        .map_err(|e| format!("Invalid base64 nonce: {}", e))?;

    // Convert the decoded bytes to a fixed-size array first: this validates
    // the length (must be exactly 12 bytes) using std's TryFrom, then
    // From<[u8; N]> builds the nonce (replaces the deprecated from_slice)
    let nonce_arr: [u8; NONCE_SIZE] = nonce_bytes.as_slice().try_into()
        .map_err(|_| format!("Invalid nonce length: expected {} bytes", NONCE_SIZE))?;
    let nonce = Nonce::from(nonce_arr);

    let plaintext = cipher
        .decrypt(&nonce, ciphertext.as_ref())
        .map_err(|e| format!("Decryption failed: {}", e))?;

    String::from_utf8(plaintext)
        .map_err(|e| format!("Decrypted text is not valid UTF-8: {}", e))
}

/// Returns a masked version of the API key for safe display.
/// Shows only the first 4 and last 4 characters: "sk-a...f456"
pub fn mask_api_key(key: &str) -> String {
    if key.len() <= 8 {
        return "****".to_string();
    }
    format!("{}...{}", &key[..4], &key[key.len()-4..])
}