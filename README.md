# Mod cộng đồng của Vityaz Hero

Nơi mọi người nộp mod. Mod được duyệt sẽ có trong danh sách mod của game.

**Hiện chỉ nhận mod không có mã:** bảng số liệu và giao diện. Mod có mã chạy với mọi quyền trên máy người chơi, nên không nhận ở đây.

## Nộp một mod

1. Làm mod từ [mod-template](https://github.com/Ink-vityaz/mod-template), thử trong game.
2. Fork kho này, chép thư mục mod vào `mods/<id>/`. Có thể tải lên bằng trình duyệt: **Add file**, **Upload files**.
3. Mở pull request. Máy kiểm ngay và nói chỗ sai.
4. Người duyệt thử trong game rồi gộp.

Cập nhật: sửa thư mục của bạn, tăng `version` trong `bundle.json`, mở pull request mới.

## Một mod phải

| | |
|---|---|
| Thư mục | `mods/<id>/`, có `bundle.json` và `README.md` (mod là gì, tác giả) |
| File | chỉ `json`, `png`, `csv`, `md`, `txt` |
| Cỡ | 512 KB mỗi file, 2 MB mỗi mod, 100 file |
| `bundle.json` | `version` ba số, `name` tới 40 chữ, `game` nói rõ bản (như `0.29.x`), `depends` trống |
| Bảng | không có dòng `button` |
| Giao diện | màu, cỡ chữ, ảnh, nền; chưa nhận phông, cảnh (`decor`) và shader |
| Ảnh, chữ | của bạn, hoặc được phép dùng và ghi nguồn trong `README.md`. Không phản cảm, không quảng cáo, không giả làm game hay người khác |

Cẩm nang: [vityaz.ink.io.vn/mod](https://vityaz.ink.io.vn/mod/).

## Giấy phép

Nộp mod là bạn đồng ý phát hành file của nó theo MIT ([LICENSE](LICENSE)), ảnh và chữ theo [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/), và xác nhận bạn có quyền làm vậy.

## Báo mod xấu

Mở một issue, ghi id của mod và chuyện gì xảy ra. Mod xấu được gỡ khỏi `mods/` và ghi vào `withdrawn.json`.

## Tự kiểm

Cần Rust. `cargo run --release --manifest-path tools/mods_community/Cargo.toml -- check`
