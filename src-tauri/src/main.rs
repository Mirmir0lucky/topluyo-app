#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::time::Duration;
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(update_loop(handle));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Topluyo başlatılamadı");
}

// Açılışta ve her saat başı güncelleme kontrol eder.
async fn update_loop(app: AppHandle) {
    loop {
        if let Err(e) = check_and_install(&app).await {
            eprintln!("Güncelleme kontrolü başarısız: {e}");
        }
        tokio::time::sleep(Duration::from_secs(60 * 60)).await;
    }
}

async fn check_and_install(app: &AppHandle) -> Result<(), tauri_plugin_updater::Error> {
    let updater = app.updater()?;
    if let Some(update) = updater.check().await? {
        update.download_and_install(|_, _| {}, || {}).await?;
        app.restart();
    }
    Ok(())
}
