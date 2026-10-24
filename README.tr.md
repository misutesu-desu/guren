# Guren

[English](README.md) · Türkçe

Rust ve egui ile yazılmış hızlı bir fotoğraf görüntüleyici. Fotoğraflar arka plandaki iş parçacıklarında çözülür, açık olanın iki yanındaki ikişer fotoğraf önceden hazırlanır. Geçişte yakınlaştırma ve kaydırma korunur, art arda çekilmiş kareleri aynı noktadan karşılaştırabilirsiniz.

JPEG, PNG, WebP, GIF, BMP, TIFF ve ICO açar. EXIF yön bilgisini uygular.

## İndirme

`guren.exe` dosyasını [son sürümden](https://github.com/misutesu-desu/guren/releases/latest) indirin. Kurulum gerekmez.

## Varsayılan görüntüleyici

Sağ üstteki ⋯ menüsünden **Varsayılan fotoğraf görüntüleyici yap**'ı seçin. Guren kendini kullanıcı hesabınıza kaydeder, yönetici izni gerekmez, ardından Windows'un Varsayılan uygulamalar sayfası açılır. Windows programların kendini varsayılan yapmasına izin vermez, bu yüzden son adımda Guren'i orada seçin. Aynı menüdeki **Birlikte aç listesinden kaldır** kaydı geri alır. `guren.exe`'yi başka yere taşırsanız bir sonraki açılışta kayıtlı yol güncellenir.

## Derleme

```bash
cargo build --release
```

Çıktı `target/release/guren.exe`. Bir dosyayı ya da klasörü argüman olarak verebilir, pencereye sürükleyip bırakabilir ya da Ctrl+O ile seçebilirsiniz. Bir dosya açılınca bulunduğu klasördeki bütün fotoğraflar sıraya girer.

## Kısayollar

| Tuş | İş |
|---|---|
| → · Boşluk · Page Down | Sonraki |
| ← · Backspace · Page Up | Önceki |
| Home · End | İlk · son |
| Fare tekerleği | İmlecin olduğu noktaya yakınlaştır |
| Sürükle · ↑ ↓ | Kaydır |
| + · − | Yakınlaştır · uzaklaştır |
| 0 | Pencereye sığdır |
| 1 | Gerçek boyut |
| Çift tıklama | Sığdır ile gerçek boyut arasında geç |
| F · F11 · Esc | Tam ekran |
| T | Tema: sistem, koyu, açık |
| L | Dil: İngilizce, Türkçe |
| Ctrl+O | Aç |

Farenin geri ve ileri tuşları da fotoğraflar arasında geçer. Arayüz sistem dili Türkçeyse Türkçe, değilse İngilizce açılır.
