fn main() {
  let _ = dotenvy::from_filename("../.env");
  if let Ok(salt) = std::env::var("WISP_SALT") {
      println!("cargo:rustc-env=WISP_SALT={}", salt);
  }
  tauri_build::build()
}
