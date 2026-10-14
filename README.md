<p align="center">
  <img src="dist/logo.png" alt="Topluyo" width="96" />
</p>

<h1 align="center">Topluyo Desktop</h1>

<p align="center">
  <a href="https://topluyo.com">topluyo.com</a> için masaüstü uygulaması. Rust ve <a href="https://tauri.app">Tauri</a> ile yazıldı.
</p>

## Özellikler

- topluyo.com'u masaüstü penceresinde açar
- Sistem tepsisi: pencere kapatılınca tepsiye küçülür (ayarlanabilir), tepsi simgesinden geri açılır
- Bilgisayar açılınca otomatik başlatma (ayarlanabilir)
- Otomatik güncelleme: açıkken saatte bir kontrol eder, yeni sürüm varsa sorar. Güncellemeler imzalıdır ve doğrulanmadan kurulmaz.
- Ayarlar penceresi (sol altta dişli simgesi): tepsi, başlangıç ve güncelleme ayarları, elle güncelleme denetimi
- İnternet yoksa küçük bir kart gösterir ve 15 saniyede bir otomatik yeniden dener
- Sağ tık menüsü: Kes, Kopyala, Yapıştır, Tümünü seç
- Kısayollar: Ctrl+C, Ctrl+X, Ctrl+V, Ctrl+A
- Topluyo dışındaki bağlantılar sistem tarayıcısında açılır (normal tık, Ctrl+tık ve orta tık)

## Kurulum

Hazır dosyalar [Releases](https://github.com/Mirmir0lucky/topluyo-app/releases) sayfasındadır.

### Windows

- `.msi` veya `.exe` dosyasını indirip çalıştırın.
- Dosya imzasız olduğu için Windows SmartScreen uyarabilir. **Ek bilgi → Yine de çalıştır** seçin.

### macOS

- `.dmg` dosyasını açıp Topluyo'yu Uygulamalar klasörüne sürükleyin.
- Uygulama imzasız olduğu için ilk açılışta **sağ tık → Aç** yapın.

### Ubuntu / Linux

- **AppImage önerilir.** İndirdikten sonra çalıştırılabilir yapın ve açın:

  ```bash
  chmod +x Topluyo_*.AppImage
  ./Topluyo_*.AppImage
  ```

- `.deb` paketinin otomatik güncelleme özelliği yoktur. Güncellemeyi elle yüklemeniz gerekir.
- GNOME masaüstünde tepsi simgesi için **AppIndicator** eklentisi gerekir.

## Geliştirme

### Gereksinimler

- [Node.js](https://nodejs.org/) 20 veya üstü
- [Rust](https://rustup.rs/) (stable)
- İşletim sistemine göre Tauri bağımlılıkları. Ayrıntılar: [Tauri ön koşulları](https://tauri.app/start/prerequisites/)
  - Windows: Visual Studio C++ Build Tools (WebView2 Windows 10/11 üzerinde genelde hazır gelir)
  - Ubuntu: `libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev` ve benzeri paketler

### Çalıştırma

```bash
npm install
npm run tauri dev
```

### Derleme

```bash
npm run tauri build
```

Çıktılar `src-tauri/target/release/bundle/` klasörüne düşer.

## Sürüm yayınlama

1. Değişiklikleri `main` dalına gönderin (push).
2. GitHub Actions Windows, macOS ve Ubuntu için derler, imzalar ve yeni bir Release oluşturur.
3. Kurulu uygulamalar bir sonraki kontrolde yeni sürümü görür.

Güncelleme imzalaması için gerekli değerler GitHub'ın **Settings → Secrets and variables → Actions** bölümünde tutulur:

- `TAURI_SIGNING_PRIVATE_KEY`: özel anahtar dosyasının içeriği
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: anahtar parolası

> **Önemli:** Özel anahtar (`*.key`) repoya eklenmez ve kimseyle paylaşılmaz. Anahtar değişirse, eski anahtarla kurulmuş uygulamalar yeni güncellemeleri doğrulayamaz. Bu durumda kullanıcıların uygulamayı Releases sayfasından bir kez elle yeniden kurması gerekir.

## Bilinen sorunlar

- **Google ile giriş:** Tam çalışmıyor. Google OAuth ayarları tamamlanmadı.
- **macOS ve Ubuntu:** Sürümler henüz tam olarak doğrulanmadı.
- **Çevrimdışı kontrolü:** Yalnızca internet bağlantısını kontrol eder. topluyo.com çalışmıyorsa (sunucu hatası) sayfa yine açılır.

## Lisans

Bu proje [MIT Lisansı](LICENSE) ile lisanslanmıştır.
