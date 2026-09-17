# Dev Companion 0.2.0 — hướng dẫn Windows

**Chưa hoàn tất migration chat sử dụng được trong Codex Desktop.** Bản này có backup môi trường và phục hồi file không ghi đè. Chưa gộp database/index, chưa kiểm chứng chat xuất hiện và mở lại trên máy đích. Không dùng kết quả “đã phục hồi file” làm bằng chứng migration thành công.

Dev Companion hiện hỗ trợ các workflow Windows cụ thể: archive môi trường Codex,
bundle `.bcpkg` do Beyond Compare xuất, `bookmarks.xml` không chứa secret của
SourceTree, và project/config được chọn của XAMPP. Bundle cá nhân luôn được coi
là dữ liệu nhạy cảm; phục hồi chỉ đưa file vào staging của Companion để bạn tự
đặt hoặc import.

## Backup / chuyển máy

1. Đóng Codex Desktop và mọi Codex CLI. Companion không tự đóng ứng dụng.
2. Mở **Backup môi trường**, chọn Chat + metadata, Cấu hình, Skills + plugin, Pets.
3. **Xem trước**: kiểm tra số file, dung lượng nguồn, danh sách loại trừ và lý do. Settings có dấu hiệu chứa credential bị loại nguyên file, không âm thầm chỉnh sửa.
4. **Tạo archive**. Chỉ nhận thành công sau khi hoàn tất ZIP và đọc lại kiểm tra SHA-256. Dung lượng ZIP được báo sau khi ghi.
5. Chuyển ZIP sang máy đích; giữ nguyên bản nguồn. ZIP chứa nội dung chat và tài nguyên cá nhân, không được mã hóa.

## Bundle cá nhân

- **Beyond Compare:** chọn file `.bcpkg` do `Tools > Export Settings` tạo. Companion giữ nguyên package, không chuyển license và không import tự động.
- **SourceTree:** chỉ đọc schema `bookmarks.xml` đã được kiểm chứng; phải đóng SourceTree. Repository, account, credential và license không được đưa vào bundle.
- **XAMPP:** chọn project trực tiếp dưới `htdocs` cùng bốn file cấu hình đã duyệt. Phải dừng Apache, MariaDB và process liên quan; binary, thư mục dữ liệu MariaDB, secret và log bị loại. Placement và import MariaDB vẫn thủ công.

## Phục hồi vào máy đã có dữ liệu

1. Đóng Codex trên máy đích; giữ bản ZIP nguồn.
2. Chọn **Kiểm tra ZIP môi trường** → **Xem trước đích**.
3. `new`: file mới được tạo. `identical`: đã giống hệt. `conflict`: giữ dữ liệu đích. `manual`: không tự phục hồi database/index/settings.
4. Nhập `RESTORE`. Chỉ tạo file mới, không thay file có sẵn. Nếu lỗi, rollback file vừa tạo; kết quả báo số file chưa rollback được.
5. Đăng nhập lại, cài/reconnect plugin và kiểm tra các phụ thuộc ngoài archive. Database/index, config có đường dẫn tuyệt đối, project và worktree cần xử lý riêng. Không copy database nguồn đè lên database máy đích.

Không có thao tác tự đổi đường dẫn trong SQLite/JSON/TOML hoặc trộn hai database. Máy đích chưa có `sessions/` vẫn có thể phục hồi file mới. Existing data được giữ nguyên nên không cần snapshot dữ liệu đích cho luồng create-only v2. Luồng session v1 vẫn giữ tùy chọn safety backup khi có conflict.

## Session v1 và khôi phục sau xóa

Trang **Backup & Restore** giữ hỗ trợ ZIP `manifest.json` v1. Chọn ZIP safety do module xóa tạo (`delete-manifest.json` v1) ở cùng nút inspect, chọn session, preview rồi restore.

Trang Conversations chỉ xóa **file session local được chọn**, không xóa chat khỏi Desktop/cloud. Preview danh sách/dung lượng, nhập `DELETE`; safety ZIP được tạo và kiểm chứng trước khi xóa. Khi thất bại, kết quả vẫn hiển thị outcome, số file đã xóa/khôi phục và đường dẫn safety ZIP. Lịch sử phân biệt DELETE với RESTORE.

## Dọn dữ liệu tạm

Đóng Codex → **Dọn dữ liệu tạm** → **Quét** → chọn nhóm → **Xem trước** → nhập `CLEAN`.

Chỉ dọn `CODEX_HOME/cache/remote_plugin_catalog/<16 hex>.json`, schema 1 với `fetched_at` và `plugins`. Đây là danh mục có thể tải lại; có thể cần mạng sau khi dọn. Không dọn `plugins/cache`, database/log SQLite, thư mục `tmp` nói chung, backup/quarantine, project/worktree, skills/pets hay dữ liệu không rõ độ an toàn.

File đổi nội dung/mtime, liên kết/junction, không rõ schema hoặc đang bị khóa bị bỏ qua. Kết quả báo số file xóa, bỏ qua, lỗi và tổng byte nội dung file đã xóa (không phải chênh lệch dung lượng trống toàn ổ đĩa, vốn có thể thay đổi do ứng dụng khác).

## Portable

Chạy `release/portable/dev-companion.exe`; giữ `portable-mode` cạnh exe. Settings luôn đọc/ghi tại `config/settings.json` cạnh exe khi marker tồn tại. Cờ portable trong Settings điều khiển vị trí backup/quarantine/history theo cấu hình hiện có. Không cần chạy quyền Administrator.
