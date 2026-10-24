# Guren

Rust ve egui ile yazılmış hızlı bir fotoğraf görüntüleyici. Fotoğraflar arka plandaki iş parçacıklarında çözülür, açık olanın iki yanındaki ikişer fotoğraf önceden hazırlanır. Geçişte yakınlaştırma ve kaydırma korunur, art arda çekilmiş kareleri aynı noktadan karşılaştırabilirsiniz.

JPEG, PNG, WebP, GIF, BMP, TIFF ve ICO açar. EXIF yön bilgisini uygular.

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
| Ctrl+O | Aç |

Farenin geri ve ileri tuşları da fotoğraflar arasında geçer.
