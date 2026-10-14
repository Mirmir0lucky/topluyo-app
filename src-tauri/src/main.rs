#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use tauri::menu::MenuItem;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::webview::NewWindowResponse;
use tauri::{AppHandle, Manager, Url, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_autostart::{ManagerExt as AutostartExt, MacosLauncher};
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::{Update, UpdaterExt};

const SITE_HOST: &str = "topluyo.com";

// Giriş (Google vb.) için açılan sayfalar uygulama içinde kalır.
const AUTH_HOSTS: &[&str] = &[
    "google.com",
    "github.com",
    "discord.com",
    "microsoftonline.com",
    "live.com",
    "apple.com",
];

// Sitenin sol alt tarafına eklenen dişli düğmesi. Tıklanınca topluyo-action:// adresine gider,
// Rust tarafı bunu yakalayıp ayarları açar. Site tarafına hiçbir yetki verilmez.
// Konumu değiştirmek için POSITION satırını düzenle.
const GEAR_JS: &str = r#"
(function () {
  if (!location.hostname.endsWith('topluyo.com')) return;
  const POSITION = { left: '112px', bottom: '8px' };

  function addGear() {
    if (document.getElementById('topluyo-gear')) return;
    const btn = document.createElement('button');
    btn.id = 'topluyo-gear';
    btn.type = 'button';
    btn.title = 'Ayarlar';
    btn.textContent = '⚙';
    Object.assign(btn.style, {
      position: 'fixed',
      left: POSITION.left,
      bottom: POSITION.bottom,
      zIndex: '2147483647',
      width: '32px',
      height: '32px',
      padding: '0',
      background: 'transparent',
      color: '#f2f0f3',
      border: 'none',
      fontSize: '18px',
      lineHeight: '1',
      cursor: 'pointer',
      opacity: '0.9'
    });
    btn.addEventListener('mouseenter', () => { btn.style.opacity = '1'; });
    btn.addEventListener('mouseleave', () => { btn.style.opacity = '0.9'; });
    btn.addEventListener('click', (e) => {
      e.preventDefault();
      e.stopPropagation();
      location.href = 'topluyo-action://settings';
    }, true);
    document.body.appendChild(btn);
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', addGear);
  } else {
    addGear();
  }
  setInterval(addGear, 2000);
})();
"#;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
struct Settings {
    close_to_tray: bool,
    start_on_login: bool,
    auto_update_check: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            close_to_tray: true,
            start_on_login: false,
            auto_update_check: true,
        }
    }
}

struct AppState {
    settings: Mutex<Settings>,
    pending_update: Mutex<Option<Update>>,
    postponed: Mutex<bool>,
}

// ---------- Ayar dosyası ----------

fn settings_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|dir| dir.join("settings.json"))
}

fn load_settings(app: &AppHandle) -> Settings {
    settings_path(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn write_settings(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let path = settings_path(app).ok_or("Ayar klasörü bulunamadı")?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| e.to_string())
}

// ---------- Komutlar (sadece yerel ayar ve güncelleme pencereleri kullanır) ----------

#[tauri::command]
fn get_settings(state: tauri::State<AppState>) -> Settings {
    let s = state.settings.lock().unwrap().clone();
    s
}

#[tauri::command]
fn set_settings(
    app: AppHandle,
    state: tauri::State<AppState>,
    settings: Settings,
) -> Result<(), String> {
    if settings.start_on_login {
        app.autolaunch().enable().map_err(|e| e.to_string())?;
    } else {
        app.autolaunch().disable().map_err(|e| e.to_string())?;
    }
    write_settings(&app, &settings)?;
    *state.settings.lock().unwrap() = settings;
    Ok(())
}

#[tauri::command]
fn app_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

#[tauri::command]
async fn check_update_now(app: AppHandle) -> String {
    check_for_update(app, true).await.to_string()
}

#[tauri::command]
fn pending_update_version(state: tauri::State<AppState>) -> Option<String> {
    let v = state
        .pending_update
        .lock()
        .unwrap()
        .as_ref()
        .map(|u| u.version.clone());
    v
}

#[tauri::command]
fn postpone_update(app: AppHandle, state: tauri::State<AppState>) {
    *state.postponed.lock().unwrap() = true;
    if let Some(w) = app.get_webview_window("update") {
        let _ = w.close();
    }
}

#[tauri::command]
fn close_current_window(window: tauri::WebviewWindow) {
    let _ = window.close();
}

#[tauri::command]
async fn install_update(app: AppHandle) -> Result<(), String> {
    let update = {
        let state = app.state::<AppState>();
        let taken = state.pending_update.lock().unwrap().take();
        taken
    };
    let update = update.ok_or("Bekleyen güncelleme yok")?;
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| e.to_string())?;
    app.restart()
}

// ---------- Pencereler ----------

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

fn open_settings(app: &AppHandle) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window("settings") {
        w.show()?;
        w.set_focus()?;
        return Ok(());
    }
    WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("settings.html".into()))
        .title("Topluyo Ayarları")
        .inner_size(440.0, 540.0)
        .resizable(false)
        .center()
        .build()?;
    Ok(())
}

fn open_update_window(app: &AppHandle) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window("update") {
        w.show()?;
        w.set_focus()?;
        return Ok(());
    }
    WebviewWindowBuilder::new(app, "update", WebviewUrl::App("update.html".into()))
        .title("Topluyo Güncelleme")
        .inner_size(400.0, 230.0)
        .resizable(false)
        .center()
        .always_on_top(true)
        .build()?;
    Ok(())
}

// ---------- Site ve bağlantı yönetimi ----------

fn is_site_url(url: &Url) -> bool {
    match url.host_str() {
        Some(host) => host == SITE_HOST || host.ends_with(".topluyo.com"),
        None => false,
    }
}

fn is_auth_url(url: &Url) -> bool {
    let Some(host) = url.host_str() else {
        return false;
    };
    AUTH_HOSTS
        .iter()
        .any(|h| host == *h || host.ends_with(&format!(".{}", h)))
}

fn is_local_url(url: &Url) -> bool {
    url.scheme() == "tauri"
        || url.scheme() == "about"
        || matches!(url.host_str(), Some("tauri.localhost") | Some("localhost"))
}

fn open_external(app: &AppHandle, url: &Url) {
    let _ = app.opener().open_url(url.as_str(), None::<&str>);
}

// Sitenin dişli düğmesinden gelen işlemleri çalıştırır.
// Pencere oluşturma, navigasyon olayının içinde değil ayrı bir görevde yapılır.
fn handle_action(app: &AppHandle, action: &str) {
    let app = app.clone();
    let action = action.to_string();
    tauri::async_runtime::spawn(async move {
        match action.as_str() {
            "settings" => {
                let _ = open_settings(&app);
            }
            "check_update" => {
                check_for_update(app, true).await;
            }
            _ => {}
        }
    });
}

fn create_main_window(app: &AppHandle) -> tauri::Result<()> {
    let nav_app = app.clone();
    let new_app = app.clone();
    WebviewWindowBuilder::new(app, "main", WebviewUrl::App("loader.html".into()))
        .title("Topluyo")
        .inner_size(1280.0, 800.0)
        .min_inner_size(800.0, 600.0)
        .initialization_script(GEAR_JS)
        .on_navigation(move |url| {
            if url.scheme() == "topluyo-action" {
                handle_action(&nav_app, url.host_str().unwrap_or(""));
                return false;
            }

            if is_site_url(&url) || is_local_url(&url) || is_auth_url(&url) {
                true
            } else {
                open_external(&nav_app, &url);
                false
            }
        })
        .on_new_window(move |url, _features| {
            if url.scheme() == "topluyo-action" {
                handle_action(&new_app, url.host_str().unwrap_or(""));
                return NewWindowResponse::Deny;
            }
            if is_site_url(&url) || is_auth_url(&url) {
                NewWindowResponse::Allow
            } else {
                open_external(&new_app, &url);
                NewWindowResponse::Deny
            }
        })
        .build()?;
    Ok(())
}

// ---------- Tepsi (sistem tepsisi) ----------

fn handle_menu_event(app: &AppHandle, id: &str) {
    match id {
        "show" => show_main(app),
        "settings" => {
            let _ = open_settings(app);
        }
        "check_update" => {
            let handle = app.clone();
            tauri::async_runtime::spawn(async move {
                check_for_update(handle, true).await;
            });
        }
        "quit" => app.exit(0),
        _ => {}
    }
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Göster", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Ayarlar", true, None::<&str>)?;
    let check = MenuItem::with_id(app, "check_update", "Güncellemeleri denetle", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Çıkış", true, None::<&str>)?;
    let menu = tauri::menu::Menu::with_items(app, &[&show, &settings, &check, &quit])?;

    let icon = app.default_window_icon().cloned();

    let mut tray = TrayIconBuilder::with_id("main-tray")
        .tooltip("Topluyo")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| handle_menu_event(app, event.id.as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        });

    if let Some(icon) = icon {
        tray = tray.icon(icon);
    }

    tray.build(app)?;
    Ok(())
}

// ---------- Güncelleme ----------

// Dönüş: "available" | "postponed" | "up-to-date" | "error"
async fn check_for_update(app: AppHandle, user_initiated: bool) -> &'static str {
    let updater = match app.updater() {
        Ok(u) => u,
        Err(e) => {
            eprintln!("Güncelleyici hazırlanamadı: {e}");
            return "error";
        }
    };

    match updater.check().await {
        Ok(Some(update)) => {
            let postponed = *app.state::<AppState>().postponed.lock().unwrap();
            if postponed && !user_initiated {
                return "postponed";
            }
            *app.state::<AppState>().pending_update.lock().unwrap() = Some(update);
            let _ = open_update_window(&app);
            "available"
        }
        Ok(None) => "up-to-date",
        Err(e) => {
            eprintln!("Güncelleme kontrolü başarısız: {e}");
            "error"
        }
    }
}

async fn update_loop(app: AppHandle) {
    loop {
        let enabled = app
            .state::<AppState>()
            .settings
            .lock()
            .unwrap()
            .auto_update_check;
        if enabled {
            check_for_update(app.clone(), false).await;
        }
        tokio::time::sleep(Duration::from_secs(60 * 60)).await;
    }
}

// ---------- Başlatma ----------

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_settings,
            set_settings,
            app_version,
            check_update_now,
            pending_update_version,
            install_update,
            postpone_update,
            close_current_window,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            let settings = load_settings(&handle);
            app.manage(AppState {
                settings: Mutex::new(settings),
                pending_update: Mutex::new(None),
                postponed: Mutex::new(false),
            });
            create_main_window(&handle)?;
            build_tray(app)?;
            tauri::async_runtime::spawn(update_loop(handle));
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                match window.label() {
                    "main" => {
                        let to_tray = window
                            .app_handle()
                            .state::<AppState>()
                            .settings
                            .lock()
                            .unwrap()
                            .close_to_tray;
                        if to_tray {
                            api.prevent_close();
                            let _ = window.hide();
                        }
                    }
                    "update" => {
                        // Pencere X ile kapatılırsa "Sonra" gibi davran
                        *window
                            .app_handle()
                            .state::<AppState>()
                            .postponed
                            .lock()
                            .unwrap() = true;
                    }
                    _ => {}
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("Topluyo başlatılamadı");
}
