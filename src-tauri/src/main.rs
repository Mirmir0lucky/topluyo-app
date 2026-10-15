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
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::{Update, UpdaterExt};

const SITE_HOST: &str = "topluyo.com";
const SITE_URL: &str = "https://topluyo.com/";

// Giriş sayfaları uygulama içinde açılır (Google girişi için). Diğer dış linkler tarayıcıda açılır.
const AUTH_HOSTS: &[&str] = &["accounts.google.com"];

// Google girişi sistem tarayıcısında yapılır; site girişten sonra topluyo://auth?token=... ile uygulamaya döner
const DESKTOP_LOGIN_URL: &str = "https://topluyo.com/?desktop_login=1";

// Sitenin içine eklenen script:
// - sol alttaki dişli düğmesi (topluyo-action://settings)
// - sağ tık menüsü (Kes, Kopyala, Yapıştır, Tümünü seç, bağlantıyı tarayıcıda aç)
// - Ctrl+C / X / V / A kısayolları
// - dış linkler (normal tık, Ctrl+tık, orta tık) sistem tarayıcısında açılır (topluyo-open://)
// Bunların hepsi sadece özel adres yönlendirmesi kullanır, siteye IPC yetkisi verilmez.
// Dişli konumunu değiştirmek için POSITION satırını düzenle.
const SITE_SCRIPT: &str = r#"
(function () {
  if (!location.hostname.endsWith('topluyo.com')) return;

  const POSITION = { left: '112px', bottom: '6px' };
  const AUTH_HOSTS = ['accounts.google.com'];

  function isAuthHost(h) {
    return AUTH_HOSTS.some((a) => h === a || h.endsWith('.' + a));
  }

  function go(url) {
    location.href = url;
  }

  // ---------- Güncelleme düğmesi (Rust tarafı açıp kapatır) ----------
  let updateOn = false;
  function applyUpdate() {
    let b = document.getElementById('topluyo-update');
    if (!updateOn) {
      if (b) b.remove();
      return;
    }
    if (b) return;
    b = document.createElement('button');
    b.id = 'topluyo-update';
    b.type = 'button';
    b.title = 'Güncelleme hazır';
    b.innerHTML = '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><path d="M12 4v12"/><path d="M6 11l6 6 6-6"/><path d="M5 20h14"/></svg>';
    Object.assign(b.style, {
      position: 'fixed',
      left: '154px',
      bottom: '8px',
      zIndex: '2147483647',
      width: '32px',
      height: '32px',
      padding: '0',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      background: 'transparent',
      color: '#22c55e',
      border: '1.5px solid #22c55e',
      borderRadius: '50%',
      cursor: 'pointer',
      opacity: '0.9',
      transition: 'background 0.15s, color 0.15s, opacity 0.15s'
    });
    // Üzerine gelince dolar, dişli ile aynı hafiflikte durur
    b.addEventListener('mouseenter', () => {
      b.style.background = '#22c55e';
      b.style.color = '#06140b';
      b.style.opacity = '1';
    });
    b.addEventListener('mouseleave', () => {
      b.style.background = 'transparent';
      b.style.color = '#22c55e';
      b.style.opacity = '0.9';
    });
    b.addEventListener('click', (e) => {
      e.preventDefault();
      e.stopPropagation();
      go('topluyo-action://update');
    }, true);
    document.body.appendChild(b);
  }
  window.__topluyoSetUpdate = (on) => {
    updateOn = !!on;
    applyUpdate();
  };

  // ---------- Ayarlar düğmesi ----------
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
      width: '36px',
      height: '36px',
      padding: '0',
      background: 'transparent',
      color: '#f2f0f3',
      border: 'none',
      fontSize: '22px',
      lineHeight: '1',
      cursor: 'pointer',
      opacity: '0.9'
    });
    btn.addEventListener('mouseenter', () => { btn.style.opacity = '1'; });
    btn.addEventListener('mouseleave', () => { btn.style.opacity = '0.9'; });
    btn.addEventListener('click', (e) => {
      e.preventDefault();
      e.stopPropagation();
      go('topluyo-action://settings');
    }, true);
    document.body.appendChild(btn);
  }

  // ---------- Dış bağlantılar ----------
  function externalLinkOf(el) {
    const a = el && el.closest ? el.closest('a[href]') : null;
    if (!a) return null;
    let u;
    try { u = new URL(a.href, location.href); } catch (e) { return null; }
    if (u.protocol !== 'http:' && u.protocol !== 'https:') return null;
    if (u.hostname.endsWith('topluyo.com')) return null;
    if (isAuthHost(u.hostname)) return null;
    return u.href;
  }

  function openExternal(url) {
    go('topluyo-open://open?u=' + encodeURIComponent(url));
  }

  document.addEventListener('click', (e) => {
    const url = externalLinkOf(e.target);
    if (url) {
      e.preventDefault();
      e.stopPropagation();
      openExternal(url);
    }
  }, true);

  document.addEventListener('auxclick', (e) => {
    if (e.button !== 1) return;
    const url = externalLinkOf(e.target);
    if (url) {
      e.preventDefault();
      e.stopPropagation();
      openExternal(url);
    }
  }, true);

  // ---------- Kısayollar ----------
  // Yapıştırma olayını izle: WebView kendi yapıştırmasını yaptıysa (resim dahil) yedeğe gerek yok
  let pasteSeen = false;
  document.addEventListener('paste', () => { pasteSeen = true; }, true);
  document.addEventListener('keydown', (e) => {
    if (!(e.ctrlKey || e.metaKey) || e.altKey) return;
    const k = (e.key || '').toLowerCase();
    if (k === 'c') {
      e.preventDefault(); e.stopPropagation(); document.execCommand('copy');
    } else if (k === 'x') {
      e.preventDefault(); e.stopPropagation(); document.execCommand('cut');
    } else if (k === 'a') {
      e.preventDefault(); e.stopPropagation(); document.execCommand('selectAll');
    } else if (k === 'v') {
      // Yapıştırmayı WebView'a bırak (resim dahil). Olay gelmezse metni Rust tarafından yapıştır.
      pasteSeen = false;
      setTimeout(() => { if (!pasteSeen) go('topluyo-action://paste'); }, 150);
    }
  }, true);

  // ---------- Sağ tık menüsü ----------
  let menuEl = null;

  function closeMenu() {
    if (menuEl) {
      menuEl.remove();
      menuEl = null;
    }
  }

  function menuItem(label, onClick) {
    const b = document.createElement('div');
    b.textContent = label;
    Object.assign(b.style, {
      padding: '7px 14px',
      borderRadius: '6px',
      cursor: 'pointer',
      whiteSpace: 'nowrap',
      background: 'transparent'
    });
    b.addEventListener('mouseenter', () => { b.style.background = '#2a2530'; });
    b.addEventListener('mouseleave', () => { b.style.background = 'transparent'; });
    // Menüye tıklarken metin seçimi ve imleç kaybolmasın
    b.addEventListener('mousedown', (e) => { e.preventDefault(); });
    b.addEventListener('click', (e) => {
      e.stopPropagation();
      closeMenu();
      onClick();
    });
    return b;
  }

  function separator() {
    const s = document.createElement('div');
    Object.assign(s.style, { height: '1px', background: '#2c2830', margin: '5px 6px' });
    return s;
  }

  document.addEventListener('contextmenu', (e) => {
    e.preventDefault();
    e.stopPropagation();
    closeMenu();

    const link = externalLinkOf(e.target);
    const menu = document.createElement('div');
    menuEl = menu;
    Object.assign(menu.style, {
      position: 'fixed',
      left: e.clientX + 'px',
      top: e.clientY + 'px',
      zIndex: '2147483647',
      background: '#1c1a1e',
      color: '#f2f0f3',
      border: '1px solid #2c2830',
      borderRadius: '10px',
      padding: '6px',
      minWidth: '190px',
      boxShadow: '0 8px 24px rgba(0,0,0,0.45)',
      fontFamily: '"Segoe UI", system-ui, sans-serif',
      fontSize: '13px',
      userSelect: 'none'
    });

    if (link) {
      menu.appendChild(menuItem('Bağlantıyı tarayıcıda aç', () => openExternal(link)));
      menu.appendChild(separator());
    }
    menu.appendChild(menuItem('Kes', () => document.execCommand('cut')));
    menu.appendChild(menuItem('Kopyala', () => document.execCommand('copy')));
    menu.appendChild(menuItem('Yapıştır', () => go('topluyo-action://paste')));
    menu.appendChild(separator());
    menu.appendChild(menuItem('Tümünü seç', () => document.execCommand('selectAll')));
    menu.appendChild(separator());
    menu.appendChild(menuItem('Sayfa bağlantısını kopyala', () => go('topluyo-copy://url?u=' + encodeURIComponent(location.href))));

    document.body.appendChild(menu);

    // Ekran dışına taşarsa içeri çek
    const r = menu.getBoundingClientRect();
    if (r.right > window.innerWidth) menu.style.left = (window.innerWidth - r.width - 4) + 'px';
    if (r.bottom > window.innerHeight) menu.style.top = (window.innerHeight - r.height - 4) + 'px';
  }, true);

  document.addEventListener('mousedown', (e) => {
    if (menuEl && !menuEl.contains(e.target)) closeMenu();
  }, true);
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') closeMenu();
  }, true);
  window.addEventListener('blur', closeMenu);
  window.addEventListener('scroll', closeMenu, true);

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', addGear);
  } else {
    addGear();
  }
  setInterval(() => { addGear(); applyUpdate(); }, 2000);
  // Sayfa hazır olunca Rust'a haber ver; Rust güncel güncelleme durumunu geri gönderir
  document.addEventListener('DOMContentLoaded', () => go('topluyo-action://ready'));
  // Sayfa boş ya da ağ hata sayfasıysa uygulamaya haber ver (bağlantı kartına dönülür)
  window.addEventListener('load', () => {
    setTimeout(() => {
      const errorPage = !!document.getElementById('main-frame-error') || (!!document.body && document.body.classList.contains('neterror'));
      const empty = !document.body || document.body.children.length === 0;
      if (errorPage || empty) go('topluyo-action://page_failed');
    }, 2500);
  });
})();
"#;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
struct Settings {
    close_to_tray: bool,
    start_on_login: bool,
    auto_update_check: bool,
    title_bar_match: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            close_to_tray: true,
            start_on_login: false,
            auto_update_check: true,
            title_bar_match: true,
        }
    }
}

struct AppState {
    settings: Mutex<Settings>,
    pending_update: Mutex<Option<Update>>,
    postponed: Mutex<bool>,
    link: Mutex<String>,
    pending_auth: Mutex<Option<String>>,
    last_failure: Mutex<Option<std::time::Instant>>,
}

// ---------- Başlık çubuğu rengi (yalnızca Windows, DWM) ----------

#[cfg(windows)]
mod titlebar {
    use std::ffi::c_void;

    #[link(name = "dwmapi")]
    extern "system" {
        fn DwmSetWindowAttribute(hwnd: *mut c_void, attr: u32, value: *const c_void, size: u32) -> i32;
    }

    const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;
    const DWMWA_CAPTION_COLOR: u32 = 35; // Windows 11
    const DWMWA_TEXT_COLOR: u32 = 36; // Windows 11
    const DWMWA_COLOR_DEFAULT: u32 = 0xFFFF_FFFF;

    fn set<T>(hwnd: *mut c_void, attr: u32, value: T) {
        unsafe {
            DwmSetWindowAttribute(
                hwnd,
                attr,
                &value as *const T as *const c_void,
                std::mem::size_of::<T>() as u32,
            );
        }
    }

    pub fn apply(window: &tauri::WebviewWindow, enabled: bool) {
        let Ok(hwnd) = window.hwnd() else {
            return;
        };
        let h = hwnd.0;
        set::<i32>(h, DWMWA_USE_IMMERSIVE_DARK_MODE, 1);
        if enabled {
            // COLORREF biçimi: 0x00BBGGRR. #141215 ve #f2f0f3
            set::<u32>(h, DWMWA_CAPTION_COLOR, 0x0015_1214);
            set::<u32>(h, DWMWA_TEXT_COLOR, 0x00F3_F0F2);
        } else {
            set::<u32>(h, DWMWA_CAPTION_COLOR, DWMWA_COLOR_DEFAULT);
            set::<u32>(h, DWMWA_TEXT_COLOR, DWMWA_COLOR_DEFAULT);
        }
    }
}

fn apply_titlebar(window: &tauri::WebviewWindow, enabled: bool) {
    #[cfg(windows)]
    titlebar::apply(window, enabled);
    #[cfg(not(windows))]
    {
        let _ = (window, enabled);
    }
}

// ---------- Tepsi simgesi varyantları ----------

// Bağlantı yoksa gri, güncelleme varsa sağ alt köşeye yeşil nokta
fn tray_icon_variant(
    base: &tauri::image::Image<'_>,
    offline: bool,
    update: bool,
) -> tauri::image::Image<'static> {
    let (w, h) = (base.width(), base.height());
    let mut rgba = base.rgba().to_vec();

    if offline {
        for px in rgba.chunks_exact_mut(4) {
            let g = (0.299 * px[0] as f32 + 0.587 * px[1] as f32 + 0.114 * px[2] as f32) as u8;
            px[0] = g;
            px[1] = g;
            px[2] = g;
        }
    }

    if update {
        let r = w.min(h) as f32 * 0.3;
        let (cx, cy) = (w as f32 - r, h as f32 - r);
        for y in 0..h {
            for x in 0..w {
                let dx = x as f32 + 0.5 - cx;
                let dy = y as f32 + 0.5 - cy;
                if dx * dx + dy * dy <= r * r {
                    let i = ((y * w + x) * 4) as usize;
                    rgba[i] = 34;
                    rgba[i + 1] = 197;
                    rgba[i + 2] = 94;
                    rgba[i + 3] = 255;
                }
            }
        }
    }

    tauri::image::Image::new_owned(rgba, w, h)
}

// ---------- Veri temizleme ----------

// Pencere açıkken WebView veri klasörü kilitli olabilir. Bu yüzden işaret dosyası bırakıp
// uygulamayı yeniden başlatır; temizlik bir sonraki açılışta, pencere oluşmadan önce yapılır.
#[tauri::command]
fn clear_site_data(app: AppHandle) -> Result<(), String> {
    let cfg = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&cfg).map_err(|e| e.to_string())?;
    std::fs::write(cfg.join("clear-site-data"), b"").map_err(|e| e.to_string())?;
    app.restart()
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
    if let Some(w) = app.get_webview_window("main") {
        apply_titlebar(&w, settings.title_bar_match);
    }
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
    set_site_update(&app, false);
    refresh_tray(&app);
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
    } else if let Some(w) = app.get_webview_window("connect") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

// Açılışta bağlantı kontrolü yapan küçük, çerçevesiz pencere.
// Bağlantı olunca site_connected komutu asıl pencereyi açar ve bu pencereyi kapatır.
fn open_connect_window(app: &AppHandle) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window("connect") {
        w.show()?;
        w.set_focus()?;
        return Ok(());
    }
    WebviewWindowBuilder::new(app, "connect", WebviewUrl::App("loader.html".into()))
        .title("Topluyo")
        .inner_size(340.0, 240.0)
        .resizable(false)
        .decorations(false)
        .center()
        .build()?;
    Ok(())
}

// Siteye gerçek bir HTTP isteği atar. Dönüş: "ok" | "server" | "offline"
fn check_site_blocking() -> &'static str {
    match ureq::get(SITE_URL).timeout(Duration::from_secs(8)).call() {
        Ok(_) => "ok",
        Err(ureq::Error::Status(code, _)) if code < 500 => "ok",
        Err(ureq::Error::Status(_, _)) => "server",
        Err(_) => "offline",
    }
}

#[tauri::command]
async fn check_site(app: AppHandle) -> String {
    set_link_status(&app, "Bağlanıyor…");
    let result = tauri::async_runtime::spawn_blocking(check_site_blocking)
        .await
        .unwrap_or("offline");
    log::info!("Site erişim kontrolü: {}", result);
    match result {
        "ok" => {}
        "server" => set_link_status(&app, "Site yanıt vermiyor"),
        _ => set_link_status(&app, "Bağlantı yok"),
    }
    result.to_string()
}

#[tauri::command]
async fn site_connected(app: AppHandle) -> Result<(), String> {
    if app.get_webview_window("main").is_none() {
        create_main_window(&app).map_err(|e| e.to_string())?;
    }
    set_link_status(&app, "Bağlı");
    if let Some(target) = app.state::<AppState>().pending_auth.lock().unwrap().take() {
        if let (Some(w), Ok(u)) = (app.get_webview_window("main"), Url::parse(&target)) {
            let _ = w.navigate(u);
        }
    }
    if let Some(w) = app.get_webview_window("main") {
        let on = app.state::<AppState>().settings.lock().unwrap().title_bar_match;
        apply_titlebar(&w, on);
    }
    if let Some(w) = app.get_webview_window("connect") {
        let _ = w.close();
    }
    Ok(())
}

fn open_settings(app: &AppHandle) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window("settings") {
        w.show()?;
        w.set_focus()?;
        return Ok(());
    }
    let w = WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("settings.html".into()))
        .title("Topluyo Ayarları")
        .inner_size(440.0, 660.0)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .center()
        .build()?;
    // Başlık çubuğu ayarlar sayfasıyla aynı koyu renk (yalnızca Windows)
    apply_titlebar(&w, true);
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

// ---------- Tepsi durumu ve güncelleme düğmesi ----------

// Tepsi ipucu: "Topluyo — Bağlı · Güncelleme hazır" gibi
fn refresh_tray(app: &AppHandle) {
    let link = app.state::<AppState>().link.lock().unwrap().clone();
    let pending = app.state::<AppState>().pending_update.lock().unwrap().is_some();
    let mut text = String::from("Topluyo");
    if !link.is_empty() {
        text.push_str(" — ");
        text.push_str(&link);
    }
    if pending {
        text.push_str(" · Güncelleme hazır");
    }
    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = tray.set_tooltip(Some(text));
        let offline = link == "Bağlantı yok" || link == "Site yanıt vermiyor" || link == "Site yüklenemedi";
        if let Some(base) = app.default_window_icon() {
            let _ = tray.set_icon(Some(tray_icon_variant(base, offline, pending)));
        }
    }
}

fn set_link_status(app: &AppHandle, status: &str) {
    *app.state::<AppState>().link.lock().unwrap() = status.to_string();
    refresh_tray(app);
}

// Sitenin yeşil güncelleme düğmesini aç/kapat
fn set_site_update(app: &AppHandle, on: bool) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.eval(&format!(
            "window.__topluyoSetUpdate && window.__topluyoSetUpdate({});",
            on
        ));
    }
}

fn open_logs(app: &AppHandle) {
    if let Ok(dir) = app.path().app_log_dir() {
        let _ = std::fs::create_dir_all(&dir);
        let _ = app
            .opener()
            .open_path(dir.to_string_lossy().to_string(), None::<&str>);
    }
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
    if url.scheme() == "http" || url.scheme() == "https" {
        let _ = app.opener().open_url(url.as_str(), None::<&str>);
    }
}

// Google girişini sistem tarayıcısında açar
fn open_login(app: &AppHandle) {
    if let Ok(url) = Url::parse(DESKTOP_LOGIN_URL) {
        open_external(app, &url);
    }
}

// Sayfa açıldı ama yüklenemedi: ana pencereyi kapatıp bağlantı kartına döner.
// Kart yeniden denediği için bağlantı geri gelince site yeni pencerede açılır.
fn handle_page_failure(app: &AppHandle) {
    {
        let state = app.state::<AppState>();
        let mut last = state.last_failure.lock().unwrap();
        if let Some(t) = *last {
            if t.elapsed() < Duration::from_secs(30) {
                return;
            }
        }
        *last = Some(std::time::Instant::now());
    }
    log::warn!("Sayfa yüklenemedi, bağlantı kartına dönülüyor");
    set_link_status(app, "Site yüklenemedi");
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.destroy();
    }
    let _ = open_connect_window(app);
}

// ---------- Derin bağlantı (topluyo://) ----------

// topluyo://auth?token=... → siteye giriş jetonunu iletir (site /auth/desktop adresinde karşılar)
fn handle_deep_link(app: &AppHandle, raw: &str) {
    let Ok(url) = Url::parse(raw) else {
        return;
    };
    if url.scheme() != "topluyo" || url.host_str() != Some("auth") {
        return;
    }
    let Some((_, token)) = url.query_pairs().find(|(k, _)| k == "token") else {
        return;
    };
    let Ok(mut target) = Url::parse(SITE_URL).and_then(|base| base.join("auth/desktop")) else {
        return;
    };
    target.query_pairs_mut().append_pair("token", &token);

    match app.get_webview_window("main") {
        Some(w) => {
            let _ = w.navigate(target);
            let _ = w.set_focus();
        }
        None => {
            // Ana pencere henüz yoksa bağlantı kurulunca açılışta uygulanır
            *app.state::<AppState>().pending_auth.lock().unwrap() = Some(target.to_string());
        }
    }
    log::info!("Giriş bağlantısı alındı");
}

// Sitenin script'inden gelen işlemler. Pencere ve clipboard işi navigasyon olayının dışında yapılır.
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
            "page_failed" => {
                handle_page_failure(&app);
            }
            "ready" => {
                let on = app.state::<AppState>().pending_update.lock().unwrap().is_some();
                set_site_update(&app, on);
            }
            "update" => {
                let has = app.state::<AppState>().pending_update.lock().unwrap().is_some();
                if has {
                    let _ = open_update_window(&app);
                }
            }
            "paste" => {
                if let Ok(text) = app.clipboard().read_text() {
                    if let Some(w) = app.get_webview_window("main") {
                        let quoted = serde_json::to_string(&text).unwrap_or_else(|_| "\"\"".into());
                        let js = format!("document.execCommand('insertText', false, {});", quoted);
                        let _ = w.eval(&js);
                    }
                }
            }
            _ => {}
        }
    });
}

fn create_main_window(app: &AppHandle) -> tauri::Result<()> {
    let nav_app = app.clone();
    let new_app = app.clone();
    let site: Url = SITE_URL.parse().expect("geçersiz site adresi");
    WebviewWindowBuilder::new(app, "main", WebviewUrl::External(site))
        .title("Topluyo")
        .inner_size(1280.0, 800.0)
        .min_inner_size(800.0, 600.0)
        .initialization_script(SITE_SCRIPT)
        .on_navigation(move |url| {
            if url.scheme() == "topluyo-action" {
                handle_action(&nav_app, url.host_str().unwrap_or(""));
                return false;
            }

            if url.scheme() == "topluyo-copy" {
                if let Some((_, text)) = url.query_pairs().find(|(k, _)| k == "u") {
                    let _ = nav_app.clipboard().write_text(text.to_string());
                }
                return false;
            }

            if url.scheme() == "topluyo-open" {
                if let Some((_, target)) = url.query_pairs().find(|(k, _)| k == "u") {
                    if let Ok(target_url) = Url::parse(&target) {
                        if is_auth_url(&target_url) {
                            open_login(&nav_app);
                        } else {
                            open_external(&nav_app, &target_url);
                        }
                    }
                }
                return false;
            }

            if is_auth_url(&url) {
                open_login(&nav_app);
                return false;
            }

            if is_site_url(&url) || is_local_url(&url) {
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
            if is_auth_url(&url) {
                open_login(&new_app);
                return NewWindowResponse::Deny;
            }
            if is_site_url(&url) {
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
        "logs" => open_logs(app),
        "quit" => app.exit(0),
        _ => {}
    }
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Göster", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Ayarlar", true, None::<&str>)?;
    let check = MenuItem::with_id(app, "check_update", "Güncellemeleri denetle", true, None::<&str>)?;
    let logs = MenuItem::with_id(app, "logs", "Günlük dosyasını aç", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Çıkış", true, None::<&str>)?;
    let menu = tauri::menu::Menu::with_items(app, &[&show, &settings, &check, &logs, &quit])?;

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
            log::info!("Yeni sürüm bulundu: {}", update.version);
            *app.state::<AppState>().pending_update.lock().unwrap() = Some(update);
            set_site_update(&app, true);
            refresh_tray(&app);
            let postponed = *app.state::<AppState>().postponed.lock().unwrap();
            if postponed && !user_initiated {
                return "postponed";
            }
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
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            for arg in argv {
                if arg.starts_with("topluyo://") {
                    handle_deep_link(app, &arg);
                }
            }
            show_main(app);
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_denylist(&["connect"])
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .invoke_handler(tauri::generate_handler![
            get_settings,
            set_settings,
            app_version,
            check_update_now,
            pending_update_version,
            install_update,
            postpone_update,
            close_current_window,
            site_connected,
            check_site,
            clear_site_data,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            let _ = app.deep_link().register("topluyo");
            for arg in std::env::args() {
                if arg.starts_with("topluyo://") {
                    handle_deep_link(&handle, &arg);
                }
            }
            if let Ok(cfg) = app.path().app_config_dir() {
                let marker = cfg.join("clear-site-data");
                if marker.exists() {
                    let bases = [app.path().app_local_data_dir(), app.path().app_data_dir()];
                    for base in bases.into_iter().flatten() {
                        let _ = std::fs::remove_dir_all(base.join("EBWebView"));
                    }
                    let _ = std::fs::remove_file(&marker);
                    log::info!("Oturum ve önbellek temizlendi");
                }
            }
            let settings = load_settings(&handle);
            app.manage(AppState {
                settings: Mutex::new(settings),
                pending_update: Mutex::new(None),
                postponed: Mutex::new(false),
                link: Mutex::new(String::new()),
                pending_auth: Mutex::new(None),
                last_failure: Mutex::new(None),
            });
            log::info!("Topluyo {} başlatıldı", handle.package_info().version);
            open_connect_window(&handle)?;
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
