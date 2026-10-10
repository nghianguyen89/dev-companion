# Dev Companion 0.2.0 — hướng dẫn Windows

**Chat trên máy đích vẫn cần kiểm tra trong Codex Desktop.** Backup môi trường phục hồi file không ghi đè; mục Di chuyển tài khoản Codex chép đè file từ ZIP, gồm database/index và danh sách dự án. Công cụ không gộp database. Không dùng kết quả “đã phục hồi file” làm bằng chứng chat đã xuất hiện và mở được trên máy đích.

Dev Companion hiện hỗ trợ các workflow Windows cụ thể: archive môi trường Codex,
bundle `.bcpkg` do Beyond Compare xuất, bookmark/cấu hình cục bộ SourceTree,
và project/config được chọn của XAMPP. Bundle cá nhân luôn được coi
là dữ liệu nhạy cảm; Beyond Compare, bookmark SourceTree và XAMPP chỉ phục hồi
vào staging để bạn tự đặt hoặc import. Cấu hình SourceTree là ngoại lệ có kiểm
tra: chỉ được ghi sau khi xác nhận và tạo bản sao an toàn.

## Hướng dẫn và Thông tin trong ứng dụng

Dưới **Cài đặt**, mở **Hướng dẫn** để xem thao tác ngắn cho từng công cụ đang
có. Mở **Thông tin** để xem phiên bản và nhật ký thay đổi của Dev Companion.

## Bảng điều khiển

**Bảng điều khiển** nằm đầu thanh bên. Trang này hiển thị số lượng môi trường,
kỹ năng, thú cưng, các nhóm chức năng, và inventory chỉ đọc của hai thư mục
backup. Tổng số file/dung lượng tính trên toàn bộ file trực tiếp; danh sách chỉ
hiện tối đa 8 file mới nhất mỗi thư mục. Nút **Làm mới** đọc lại metadata tên,
dung lượng và thời gian sửa của file, không mở hay sửa nội dung bundle.

## File Transfer (Robocopy)

Chọn **File Transfer** để chuyển thư mục cục bộ bằng `robocopy.exe` trực tiếp.
Nút Browse nằm ở đầu từng panel Source/Destination để chọn đường dẫn trước khi
xem nội dung. Click tên folder để duyệt thư mục con; breadcrumb và Up quay lại.
Source đổi thư mục sẽ xóa selection cũ; Destination chỉ đọc. Danh sách hiện các
mục direct-child, gồm hidden/system, với icon và màu có legend. Mặc định Source
không chọn mục nào: chỉ thư mục/file được tick mới được chuyển. Nếu tick một
thư mục thì toàn bộ nội dung bên trong thư mục đó được copy (trừ exclusion đã
thêm); các mục cùng cấp chưa tick sẽ không được chuyển.
Xem command preview trước khi chạy; nút Analyze thêm `/L`, nên không copy hay
xóa dữ liệu. Simple Copy dùng `/E`; Fast Copy thêm `/MT:8`; Project Migration
thêm các loại trừ build/cache (`node_modules`, `dist`, `build`, `coverage`,
`.cache`, `.next`, `.nuxt`, `.vite`, `.tmp`, `temp`, `bin`, `obj`) nhưng giữ
`.git`, `.env` và IDE metadata. Có thể sửa danh sách này.

Mirror dùng `/MIR` và có thể xóa nội dung chỉ có ở đích. Mỗi lần chạy đều phải
xác nhận; nếu đích đã có dữ liệu cần xác nhận lần hai. Đường dẫn Windows,
Program Files, ProgramData và root ổ đĩa cũng cần xác nhận rõ ràng. Không có
Move hoặc Pause: Cancel dừng Robocopy, sau đó Start lại để Robocopy tự bỏ qua
file hoàn tất theo ngữ nghĩa của nó.

Start chạy Analyze `/L` trước. Khi Analyze có tổng bytes và Robocopy output
English có dòng file hoàn tất hợp lệ, thanh progress hiện percent thực; nếu
thiếu một trong hai thì hiển thị indeterminate rõ ràng, không ước lượng. Output
được stream trực tiếp. Parser chỉ hiển thị số file/byte khi Robocopy có summary
phù hợp; các giá trị không chắc chắn là “Estimate unavailable”. Exit code 0–1
là success, 2–7 là completed-with-warning, và từ 8 là failure. Khi bật
Destination verification, Companion chạy một dry-run thứ hai, không dùng
hash/checksum và không thực hiện thao tác phá hủy. Logs nằm trong config
portable/app-data hiện có tại `logs/file-transfer`; history giữ tối đa 100 mục;
profiles được lưu cùng settings cục bộ.

## Backup / chuyển máy

### Di chuyển nhiều tài khoản Codex

1. Đóng Codex Desktop và mọi Codex CLI, mở **Codex > Di chuyển tài khoản Codex**. Nếu còn tiến trình nền, bấm **Đóng tiến trình Codex**, xác nhận, rồi kiểm tra lại; thao tác này hủy công việc Codex đang chạy.
2. Ứng dụng tự phát hiện các thư mục trực tiếp `.codex` và `.codex-*` (ví dụ
   `.codex-cus`) rồi tick sẵn mọi nhóm dữ liệu bền vững: Chat/database,
   settings an toàn + `AGENTS.md`, skills, pets, dữ liệu Project ChatGPT cục bộ,
   trạng thái Codex bổ sung, worktrees, plugins và visualizations. Project là
   `.chatgpt-projects` bên trong account, gồm metadata, hướng dẫn và source cục
   bộ; không đảm bảo dựng lại trạng thái Project cloud. Mỗi lựa chọn và tổng
   đang tick đều hiển thị dung lượng ước tính từ metadata nguồn hiện tại.
3. Nhấn **Tạo backup**. Ứng dụng báo lúc đang rà soát/tạo ZIP và báo hoàn tất
   ngay dưới nút. ZIP được kiểm SHA-256 sau khi ghi; `auth.json`, định danh
   máy, cache, temporary/runtime, sandbox, metadata Git và settings có dấu
   hiệu credential không bao giờ được gồm.
4. Mục **Các bản backup đã tạo** cho biết số ZIP, thời gian và dung lượng. Có
   thể mở Explorer tại ZIP hoặc bấm **Xóa** và xác nhận để xóa đúng ZIP đó.
5. Trên máy mới, copy ZIP vào thư mục `backups/`, rồi bấm **Khôi phục** ngay
   trên dòng ZIP trong danh sách để kiểm tra và xem trước. Nhập `REPLACE CODEX`,
   rồi bấm **Thay dữ liệu Codex**: công cụ chép đè toàn bộ file trong backup,
   gồm danh sách dự án, trạng thái và database. Codex phải đóng; Companion tạo
   và kiểm chứng ZIP safety dữ liệu đích trước khi ghi. Snapshot chat được thay
     hoàn toàn để tránh WAL/SHM cũ; file riêng của máy đích ở nhóm khác được giữ.
     File đích dùng cơ chế ghi thông thường của Windows, kiểm tra kích thước và
     SHA-256 trên cùng handle trước khi đóng; không ép đồng bộ xuống đĩa cho
     từng file. ZIP safety vẫn được đồng
     bộ và kiểm chứng trước khi ghi đè. Giữ ZIP nguồn/safety đến khi kiểm tra
     Codex xong; hash khớp không bảo đảm dữ liệu đích đã xuống đĩa khi mất điện.
   Không merge SQLite hay so sánh ngày sửa. Credential, cache và dữ liệu tạm
   không nằm trong backup. Đăng nhập lại Codex và kiểm tra chat/project sau khi
   phục hồi. ZIP safety được giữ tại đường dẫn hiển thị để có thể phục hồi lại
   qua kiểm tra ZIP; danh sách backup di chuyển chỉ liệt kê ZIP di chuyển thông thường.

Mỗi lần thử thay dữ liệu tạo một `codex-safety-*.zip` riêng. Sau khi đóng
Companion, anh có thể xóa các bản safety trung gian để giảm dung lượng; giữ ZIP
di chuyển gốc, bản safety đầu tiên và mới nhất đến khi kiểm tra phục hồi xong.
Không xóa safety đang được dùng bởi thao tác phục hồi.

Tiến độ cho biết file hiện tại, đang đọc ZIP hay ghi file đích và số byte đã ghi
trên kích thước dự kiến. Byte đã ghi chưa thay cho bước kiểm chứng SHA-256;
chỉ mở Codex sau khi kết quả phục hồi báo hoàn tất.

### Backup môi trường v2 cũ

1. Đóng Codex Desktop và mọi Codex CLI. Companion không tự đóng ứng dụng.
2. Mở **Backup môi trường**, chọn Chat + metadata, Cấu hình, Skills + plugin, Pets.
3. **Xem trước**: kiểm tra số file, dung lượng nguồn, danh sách loại trừ và lý do. Settings có dấu hiệu chứa credential bị loại nguyên file, không âm thầm chỉnh sửa.
4. **Tạo archive**. Chỉ nhận thành công sau khi hoàn tất ZIP và đọc lại kiểm tra SHA-256. Dung lượng ZIP được báo sau khi ghi.
5. Chuyển ZIP sang máy đích; giữ nguyên bản nguồn. ZIP chứa nội dung chat và tài nguyên cá nhân, không được mã hóa.

## Bundle cá nhân

- **Beyond Compare:** chọn file `.bcpkg` do `Tools > Export Settings` tạo. Companion giữ nguyên package, không chuyển license và không import tự động.
- **SourceTree bookmark:** luồng tương thích chỉ đọc schema `bookmarks.xml` đã được kiểm chứng; phải đóng SourceTree. Repository, account, credential và license không được đưa vào bundle.
- **SourceTree cấu hình cá nhân:** đóng SourceTree, mở trang **SourceTree**, nhập mật khẩu bundle tối thiểu 6 ký tự rồi chọn tạo bundle. Mục **Các bundle cấu hình đã tạo** cho biết tên, thời gian và dung lượng; có thể mở Explorer đúng file hoặc xóa sau khi xác nhận. Bundle AES-256 có thể gồm các file cục bộ phát hiện được: `accounts.json`, `bookmarks.xml`, `customactions.xml`, `hostedaccounts.xml`, `opentabs.xml`, `passwd`, `userhosts` và `user.config`. Công cụ chép đè file cấu hình chính vào cả `%LOCALAPPDATA%\Atlassian\SourceTree` và `%APPDATA%\Atlassian\SourceTree`, với bản sao an toàn riêng cho từng đường dẫn, để profile đích nào cũng nhận đủ bookmarks, tabs, passwd và userhosts. `user.config` được xác định riêng theo cài đặt hiện tại; profile nguồn trong bundle chỉ dùng làm thông tin xuất xứ. Khi phục hồi, bấm **Phục hồi cấu hình** ngay trên bundle cần dùng, nhập mật khẩu trong dòng bundle, chờ kiểm chứng, bấm **Xem trước phục hồi**, nhập `RESTORE` rồi bấm **Phục hồi cấu hình**; file đích có bản sao an toàn trước khi thay thế và kết quả liệt kê từng đường dẫn đã hash-verify. Chỉ để trống mật khẩu với bundle cũ chưa mã hóa. Windows Credential Manager, OAuth/DPAPI và SSH key không nằm trong bundle nên có thể vẫn phải đăng nhập lại.
- **XAMPP:** chọn project trực tiếp dưới `htdocs` cùng bốn file cấu hình đã duyệt. Phải dừng Apache, MariaDB và process liên quan; binary, thư mục dữ liệu MariaDB, secret và log bị loại. Placement và import MariaDB vẫn thủ công.

## Kỹ năng, thú cưng và nén tệp

- **Kỹ năng:** chỉ đọc metadata `SKILL.md`; nhập/xuất luôn tạo thư mục mới và
  không ghi đè. Nội dung kỹ năng không hiện trong ứng dụng.
- **Thú cưng:** chỉ nhận pet v2 hợp lệ kèm sprite PNG/WebP; cài đặt tạo thư mục
  mới, gỡ yêu cầu nhập `REMOVE`.
- **Nén tệp:** cần 7-Zip cục bộ. Khi chọn nguồn, dependency/build output/cache/log
  phổ biến được bỏ chọn sẵn; có thể chọn lại từng mục trên cây file chỉ đọc. Đặt
  exclusion bổ sung rồi chọn Fast hoặc Strong. Tiến độ chỉ hiện phần trăm khi 7-Zip
  báo dữ liệu đáng tin cậy; có thể Cancel, không ghi đè archive đã tồn tại.

## Phục hồi vào máy đã có dữ liệu

1. Đóng Codex trên máy đích; giữ bản ZIP nguồn.
2. Chọn **Kiểm tra ZIP môi trường** → **Xem trước đích**.
3. `new`: file mới được tạo. `identical`: đã giống hệt. `conflict`: giữ dữ liệu đích. `manual`: không tự phục hồi database/index/settings.
4. Nhập `RESTORE`. Chỉ tạo file mới, không thay file có sẵn. Nếu lỗi, rollback file vừa tạo; kết quả báo số file chưa rollback được.
5. Đăng nhập lại, cài/reconnect plugin và kiểm tra các phụ thuộc ngoài archive. Database/index, config có đường dẫn tuyệt đối, project và worktree cần xử lý riêng. Không copy database nguồn đè lên database máy đích.

Không có thao tác tự đổi đường dẫn trong SQLite/JSON/TOML hoặc trộn hai database. Máy đích chưa có `sessions/` vẫn có thể phục hồi file mới. Existing data được giữ nguyên nên không cần snapshot dữ liệu đích cho luồng create-only v2. Luồng session v1 vẫn giữ tùy chọn safety backup khi có conflict.

## Xóa session legacy cục bộ

Trang Conversations chỉ xóa **file session local được chọn**, không xóa chat khỏi Desktop/cloud. Preview danh sách/dung lượng, nhập `DELETE`; safety ZIP được tạo và kiểm chứng trước khi xóa. Khi thất bại, ứng dụng rollback ngay từ safety ZIP; không còn màn hình backup/restore session legacy riêng.

## Dọn dữ liệu tạm

Đóng Codex → **Dọn dữ liệu tạm** → **Quét** → chọn nhóm → **Xem trước** → nhập `CLEAN`.

Chỉ dọn `CODEX_HOME/cache/remote_plugin_catalog/<16 hex>.json`, schema 1 với `fetched_at` và `plugins`. Đây là danh mục có thể tải lại; có thể cần mạng sau khi dọn. Không dọn `plugins/cache`, database/log SQLite, thư mục `tmp` nói chung, backup/quarantine, project/worktree, skills/pets hay dữ liệu không rõ độ an toàn.

File đổi nội dung/mtime, liên kết/junction, không rõ schema hoặc đang bị khóa bị bỏ qua. Kết quả báo số file xóa, bỏ qua, lỗi và tổng byte nội dung file đã xóa (không phải chênh lệch dung lượng trống toàn ổ đĩa, vốn có thể thay đổi do ứng dụng khác).

## Portable

Chạy `release/portable/dev-companion.exe`; giữ toàn bộ thư mục cùng
`portable-mode` cạnh exe. Khi bật Portable trong Settings, Companion dùng
`configs/`, `backups/` và `quarantine/` cạnh exe cho settings, backup và safety
data. ZIP di chuyển Codex và bundle ứng dụng cá nhân cùng nằm trong `backups/`,
nên copy cả thư mục portable là mang theo toàn bộ archive. Danh sách cấu hình
SourceTree tự chuyển ZIP cũ đúng định dạng từ `backup/` sang `backups/`; nếu
trùng tên, file cũ được giữ nguyên và không bị ghi đè. Các chức năng thông thường không cần quyền Administrator; cấu hình domain XAMPP cần quyền này.

Settings chung nằm tại `configs/settings.json`, domain tại `configs/xampp/`.
Bản mới tự chuyển thư mục `config/` cũ sang `configs/`, giữ nguyên tệp; nếu
trùng tên, dừng và giữ cả hai thư mục để xử lý, không ghi đè.

## XAMPP: domain local và HTTPS

- Mở tool với quyền quản trị; phần Domain có nút mở lại bằng quyền này.
- Kiểm tra/chọn thư mục XAMPP. Apache phải trỏ đúng bản cài; nếu vừa chuyển
  XAMPP sang vị trí khác, chạy `setup_xampp.bat` của XAMPP trước.
- Bấm thiết lập lần đầu: dùng lại CA hợp lệ hoặc nhập thông tin tạo CA mới.
  Tool tin cậy CA cho máy này, cấu hình localhost và nhập domain cũ hỗ trợ.
- Nhập domain, chọn DocumentRoot (ví dụ `public`), tùy chọn www, chuyển HTTPS,
  danh sách file và LAN; lưu để Apache tự khởi động/restart. Sửa/xóa chỉ tác
  động cấu hình domain, không xóa source. localhost luôn được giữ.
- Bật/tắt danh sách file qua tùy chọn hoặc nút riêng. Mặc định domain mới tắt.
- Danh sách domain: biểu tượng SSL xanh lá khi domain bật chuyển hướng HTTPS và CA sẵn
  sàng; WWW xanh dương khi bật bí danh, trạng thái chưa bật có màu xám nhạt.
  Bỏ chọn chuyển HTTP sang HTTPS rồi lưu sẽ làm SSL xám. Nút mở và copy dùng URL
  đầy đủ của domain chính, không thêm www; dùng HTTPS khi bật chuyển hướng và CA
  sẵn sàng, HTTP khi tắt. Tắt chuyển hướng vẫn giữ chứng chỉ và truy cập HTTPS.
  Bấm đường dẫn/biểu tượng thư mục để mở source trong Explorer. Ba icon bên
  phải là bật/tắt liệt kê, sửa và xóa; rê chuột để xem tooltip. Mở/copy/thư mục
  không cần Administrator. Nút sửa cuộn đến form và cho chỉnh bản nháp; khi
  chưa có quyền quản trị, index/xóa hiện hướng dẫn và nút mở lại bằng quyền này.
  Lưu/xóa cấu hình thật vẫn cần Administrator. Copy báo thành công bằng dấu
  check ngay tại icon, không chuyển toàn bộ phần Domain sang trạng thái chờ.
- LAN: chọn mạng Windows Private; tool mở firewall Apache trong subnet nội bộ.
  Trên máy khách, trỏ domain/www qua hosts hoặc DNS về IP LAN hiển thị của máy
  chủ và cài CA công khai xuất từ tool vào Trusted Root. Không chia sẻ key CA.
- Portable lưu cấu hình tại `configs/xampp/` cạnh `backups/`. Private key nằm
  trong `xampp/apache/conf/dev-companion/`, không nằm trong backup thông thường.
- Cấu hình/file tùy chỉnh cũ và các lần sửa được lưu trước vào
  `xampp/backup/dev-companion-*/`; `recovery.json` chỉ rõ đường dẫn từng bản sao.
  File không xác định hoặc CA vẫn được cấu hình khác sử dụng được giữ lại.
- Backup XAMPP vẫn cần dừng Apache/MariaDB và process liên quan; phần Domain
  chạy độc lập với yêu cầu dừng process của backup.

### Quản lý backup cấu hình domain

Trong phần Domain có mục Bản sao lưu an toàn: số bản khôi phục hợp lệ, dung
lượng và số bản muốn giữ (mặc định 10, cho đổi từ 1 đến 100). Lưu chính sách
không restart Apache; bấm Dọn bản sao lưu cũ để dọn ngay theo chính sách đã lưu.
Tool cũng tự dọn sau khi áp dụng cấu hình/restart thành công.

Luôn giữ bản đầu tiên, các bản thiết lập đầu tiên, bản lỗi/đang xử lý và bản
được tạo bằng phiên bản cũ chưa ghi trạng thái. Số bản giữ lại chỉ giới hạn
các bản áp dụng thành công đủ điều kiện dọn; vì vậy tổng số bản có thể lớn
hơn con số đã chọn. Các backup cũ hiện có không bị gán trạng thái thành công
để tự xóa. File lạ, manifest hỏng hay liên kết/junction được giữ nguyên;
phần thống kê chỉ tính các bản có thông tin khôi phục hợp lệ, không phải tổng
dung lượng toàn bộ C:/xampp/backup. Cần quyền Administrator để lưu/dọn.
