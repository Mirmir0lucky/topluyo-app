<div align="center">

<img src="dist/logo.png" alt="Topluyo" width="112" />

# Topluyo Desktop

**Topluyo deneyimini masaüstüne taşı.**

[topluyo.com](https://topluyo.com/) için Rust ve [Tauri](https://tauri.app/) ile geliştirilen hafif, yerel masaüstü uygulaması.

[İndir](https://github.com/Mirmir0lucky/topluyo-app/releases) · [Web Sitesi](https://topluyo.com/) · [Hata Bildir](https://github.com/Mirmir0lucky/topluyo-app/issues)

  
  
  

</div>
    
---

## 📥 Kurulum

Güncel sürümü GitHub Releases sayfasından indirebilirsin.

<div align="center">

### [⬇️ Tüm sürümleri görüntüle](https://github.com/Mirmir0lucky/topluyo-app/releases)

</div>

### Windows

1. [Releases](https://github.com/Mirmir0lucky/topluyo-app/releases) sayfasını aç.
    
2. İndirilenler arasından `.msi` veya `.exe` kurulum dosyasını seç.
    
3. Kurulum dosyasını çalıştır ve ekrandaki adımları tamamla.
    

> [!WARNING]  
> Uygulama yayıncı sertifikasıyla imzalanmadığı için Windows SmartScreen güvenlik uyarısı gösterebilir.

### macOS

1. Releases sayfasından `.dmg` dosyasını indir.
    
2. Disk görüntüsünü aç.
    
3. Topluyo uygulamasını **Applications (Uygulamalar)** klasörüne sürükle.
    

> [!WARNING]  
> Uygulama Apple tarafından imzalanmadığı veya noter onayından geçirilmediği için macOS ilk açılışta güvenlik uyarısı gösterebilir.

### Ubuntu ve diğer Linux dağıtımları

**Önerilen format: AppImage**

1. Releases sayfasından `.AppImage` dosyasını indir.
    
2. Terminali aç ve dosyanın bulunduğu dizine geç.
    
3. Aşağıdaki komutları çalıştır:
    

```
chmod +x Topluyo_*.AppImage
./Topluyo_*.AppImage
```

**Debian / Ubuntu (**`**.deb**`**)**

`.deb` paketini indirip paket yöneticisiyle kurabilirsin. Bu paket için uygulama içi otomatik güncelleme desteklenmez; yeni sürümleri Releases sayfasından indirerek elle kurmalısın.

> [!TIP]  
> GNOME masaüstü ortamında sistem tepsisi simgesinin görüntülenmesi için AppIndicator desteği gerekebilir.

---

## 🛠️ Geliştirme

Projeyi yerel ortamında çalıştırmak veya geliştirmeye katkıda bulunmak için aşağıdaki adımları izleyebilirsin.

### Gereksinimler

- [Node.js](https://nodejs.org/) 20 veya üzeri
    
- [Rust](https://rustup.rs/) — stable sürümü
    
- [Tauri sistem gereksinimleri](https://tauri.app/start/prerequisites/)
    

**Windows**

- Microsoft Visual Studio C++ Build Tools
    
- WebView2 Runtime
    

**Ubuntu / Debian tabanlı Linux**

Tauri'nin ihtiyaç duyduğu WebKitGTK, AppIndicator ve diğer sistem bağımlılıklarını kurmalısın. Gerekli paketler dağıtım sürümüne göre değişebilir.

### Kurulumu tamamla

```
npm install
```

### Geliştirme modunda çalıştır

```
npm run tauri dev
```

Bu komut, uygulamayı geliştirme modunda başlatır.

### Üretim sürümünü derle

```
npm run tauri build
```

Derleme çıktıları genel olarak aşağıdaki dizinde oluşturulur:

```
src-tauri/target/release/bundle/
```

Oluşturulan kurulum paketleri hedef işletim sistemine ve Tauri yapılandırmasına göre değişir.

---

## ⚠️ Bilinen sorunlar ve sınırlamalar

- **Google ile giriş:** Google OAuth yapılandırması tamamlanmadığından giriş deneyimi henüz tam olarak çalışmıyor.
    
- **macOS ve Linux doğrulaması:** Bu platformlardaki sürümler henüz tüm senaryolar için doğrulanmış değil.
    
- **Çevrimdışı algılama:** Kontrol, internet bağlantısının durumuna dayanır. Topluyo sunucusu erişilemez durumdaysa veya hata döndürüyorsa çevrimdışı ekranı her durumda gösterilmeyebilir.
    
- **Linux otomatik güncelleme:** `.deb` paketi için uygulama içi otomatik güncelleme mevcut değildir.
    

---

<div align="center">


[Web Sitesi](https://topluyo.com/) · [GitHub Releases](https://github.com/Mirmir0lucky/topluyo-app/releases)

</div>
*yapay zeka kullanılarak yapıldı. orjinal repositery: https://github.com/topluyo/App*
