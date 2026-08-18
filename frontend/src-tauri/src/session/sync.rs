use crate::security::crypto::{decrypt_bytes, encrypt_bytes};
use std::env;
use std::fs::{self, File};
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;
use zip::write::FileOptions;
use walkdir::WalkDir;

pub async fn init_profile(
    user_id: &str,
    key: &[u8; 32],
    encrypted_blob: &[u8],
) -> Result<PathBuf, String> {
    // 1. Создаем изолированную директорию
    let temp_dir = env::temp_dir().join(format!("chabupelik_session_{}", user_id));
    if temp_dir.exists() {
        let _ = fs::remove_dir_all(&temp_dir);
    }
    fs::create_dir_all(&temp_dir).map_err(|e| format!("Failed to create temp dir: {}", e))?;

    // 2. Устанавливаем переменную для WebView2 (УДАЛЕНО, используем data_directory)

    // 3. Расшифровка и распаковка архива (если профиль уже есть)
    if !encrypted_blob.is_empty() {
        let zip_bytes = decrypt_bytes(encrypted_blob, key)?;
        
        let reader = Cursor::new(zip_bytes);
        let mut archive = zip::ZipArchive::new(reader).map_err(|e| e.to_string())?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).unwrap();
            let outpath = match file.enclosed_name() {
                Some(path) => temp_dir.join(path),
                None => continue,
            };

            if (*file.name()).ends_with('/') {
                fs::create_dir_all(&outpath).unwrap();
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        fs::create_dir_all(p).unwrap();
                    }
                }
                let mut outfile = File::create(&outpath).unwrap();
                std::io::copy(&mut file, &mut outfile).unwrap();
            }
        }
    }

    Ok(temp_dir)
}

pub async fn sync_and_cleanup(
    session_token: &str,
    key: &[u8; 32],
    temp_path: &PathBuf,
) -> Result<(), String> {
    if !temp_path.exists() {
        return Ok(());
    }

    // 1. Архивация содержимого
    let mut zip_buffer = Vec::new();
    {
        let writer = Cursor::new(&mut zip_buffer);
        let mut zip = zip::ZipWriter::new(writer);
        let options = FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o755);

        let walkdir = WalkDir::new(temp_path);
        let it = walkdir.into_iter();

        for entry in it.filter_map(|e| e.ok()) {
            let path = entry.path();
            let name = path.strip_prefix(temp_path).unwrap();
            let name_str = name.to_string_lossy().replace("\\", "/");

            if path.is_file() {
                let mut f = match File::open(path) {
                    Ok(file) => file,
                    Err(e) => {
                        println!("[SYNC] Skipping locked or inaccessible file {}: {}", path.display(), e);
                        continue;
                    }
                };
                let _ = zip.start_file(&name_str, options);
                let mut buffer = Vec::new();
                if let Err(e) = f.read_to_end(&mut buffer) {
                    println!("[SYNC] Skipping unreadable file {}: {}", path.display(), e);
                    continue;
                }
                let _ = zip.write_all(&buffer);
            } else if !name.as_os_str().is_empty() {
                let _ = zip.add_directory(&name_str, options);
            }
        }
        zip.finish().map_err(|e| e.to_string())?;
    }

    // 2. Шифрование архива ключом пользователя
    let encrypted_payload = encrypt_bytes(&zip_buffer, key)?;

    // 3. Отправка POST /api/sync
    let client = reqwest::Client::new();
    let api_domain = env::var("VITE_API_DOMAIN").unwrap_or_else(|_| "localhost:3000".to_string());
    
    // Поддержка http для локальной разработки, https для прода (зависит от VITE_API_DOMAIN)
    let url = if api_domain.starts_with("http") {
        format!("{}/api/sync", api_domain)
    } else {
        format!("https://{}/api/sync", api_domain)
    };

    let res = client.post(&url)
        .header("Authorization", format!("Bearer {}", session_token))
        .header("Content-Type", "application/octet-stream")
        .body(encrypted_payload)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        return Err(format!("Sync failed with status: {}", res.status()));
    }

    // 4. Безопасное удаление временной директории
    let _ = fs::remove_dir_all(temp_path);

    Ok(())
}
