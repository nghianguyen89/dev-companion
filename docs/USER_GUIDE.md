# Dev Companion 0.2.0 — hướng dẫn Windows

**Chat trên máy đích vẫn cần kiểm tra trong Codex Desktop.** Backup môi trường phục hồi file không ghi đè; mục Di chuyển tài khoản Codex chép đè file từ ZIP, gồm database/index và danh sách dự án. Công cụ không gộp database. Không dùng kết quả “đã phục hồi file” làm bằng chứng chat đã xuất hiện và mở được trên máy đích.

Dev Companion hiện hỗ trợ các workflow Windows cụ thể: archive môi trường Codex,
bundle `.bcpkg` do Beyond Compare xuất, bookmark/cấu hình cục bộ SourceTree,
và project/config được chọn của XAMPP. Bundle cá nhân luôn được coi
là dữ liệu nhạy cảm; Beyond Compare, bookmark SourceTree và XAMPP chỉ phục hồi
vào staging để bạn tự đặt hoặc import. Cấu hình SourceTree là ngoại lệ có kiểm
tra: chỉ được ghi sau khi xác nhận và tạo bản sao an toàn.

## Môi trường Codex

Lưu môi trường sẽ tạo launcher trong `%USERPROFILE%\bin` và tự thêm thư mục
này vào User PATH. Với môi trường đã tạo bằng bản cũ, bấm **Tạo lại launcher**,
sau đó **Kiểm tra alias**. Phép kiểm tra dùng PATH mới và chỉ chạy `--version`;
nó báo lỗi nếu thiếu launcher, trùng lệnh hoặc launcher cũ/bị sửa. Đóng rồi mở
lại các terminal đã chạy trước khi đổi PATH. Nếu chuyển phiên bản Node bằng
nvm hoặc đổi nơi cài CLI, tạo lại launcher để cập nhật đường dẫn.

**Đăng nhập: Chưa kiểm tra được** nghĩa là lệnh `codex login status` chưa cho
kết quả xác định; không phải môi trường bị hỏng. Kiểm tra trạng thái CLI phía
trên rồi bấm **Làm mới trạng thái**. Khi tìm thấy CLI nhưng kiểm tra phiên bản
thất bại, app hiển thị lỗi riêng để anh kiểm tra Node.js/CLI.

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
   settings + `AGENTS.md`, skills, pets, dữ liệu Project ChatGPT cục bộ,
   trạng thái Codex bổ sung, worktrees, plugins và visualizations. Project là
   `.chatgpt-projects` bên trong account, gồm metadata, hướng dẫn và source cục
   bộ; không đảm bảo dựng lại trạng thái Project cloud. Mỗi lựa chọn và tổng
   đang tick đều hiển thị dung lượng ước tính từ metadata nguồn hiện tại.
3. Nhấn **Tạo backup**. Ứng dụng báo lúc đang rà soát/tạo ZIP và báo hoàn tất
   ngay dưới nút. ZIP được kiểm SHA-256 sau khi ghi; `auth.json`, định danh
   máy, cache, temporary/runtime, sandbox và metadata Git không được gồm.
   Tùy chọn **Di chuyển cả nội dung nhạy cảm** mặc định tắt. Bật để giữ nguyên
   toàn bộ nội dung cấu hình đã chọn (`AGENTS.md`, `config.toml`, rules,
   prompts), kể cả file có dấu hiệu credential. Khi tắt, những file này vẫn bị
   bộ lọc nhạy cảm loại nguyên file; ví dụ `secretary` cũng khớp `secret`.
   Số file và dung lượng ước tính cập nhật theo tùy chọn. ZIP không được mã hóa;
   chỉ lưu/chuyển ở nơi tin cậy. Tùy chọn không gồm file đăng nhập Codex
   (`auth.json`); máy mới vẫn cần đăng nhập lại.
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

Đóng Codex → nhóm **Codex** → **Dọn dữ liệu tạm** → **Quét** → chọn nhóm → **Xem trước** → nhập `CLEAN`.

Sau **Quét**, bảng **Dung lượng Codex nằm ở đâu** cho biết dung lượng từng thư mục và nhóm file gốc. Bảng chỉ đọc metadata, không mở nội dung hay cho phép xóa các nhóm này. Giới hạn 50.000 mục / 32 cấp; liên kết, mục không truy cập được và giới hạn quét được báo là kết quả chưa đầy đủ. Thư mục `.tmp` có thể chứa checkout marketplace/plugin còn được tham chiếu, nên không tự coi là rác.

Chỉ dọn `CODEX_HOME/cache/remote_plugin_catalog/<16 hex>.json`, schema 1 với `fetched_at` và `plugins`. Đây là danh mục có thể tải lại; có thể cần mạng sau khi dọn. Không dọn `plugins/cache`, database/log SQLite, thư mục `tmp` nói chung, backup/quarantine, project/worktree, skills/pets hay dữ liệu không rõ độ an toàn.

File đổi nội dung/mtime, liên kết/junction, không rõ schema hoặc đang bị khóa bị bỏ qua. Kết quả báo số file xóa, bỏ qua, lỗi và tổng byte nội dung file đã xóa (không phải chênh lệch dung lượng trống toàn ổ đĩa, vốn có thể thay đổi do ứng dụng khác).

## Portable

Chạy `release/portable/dev-companion.exe`; giữ toàn bộ thư mục cùng
`portable-mode` cạnh exe. Khi bật Portable trong Settings, Companion dùng
`config/`, `backups/` và `quarantine/` cạnh exe cho settings, backup và safety
data. ZIP di chuyển Codex và bundle ứng dụng cá nhân cùng nằm trong `backups/`,
nên copy cả thư mục portable là mang theo toàn bộ archive. Danh sách cấu hình
SourceTree tự chuyển ZIP cũ đúng định dạng từ `backup/` sang `backups/`; nếu
trùng tên, file cũ được giữ nguyên và không bị ghi đè. Không cần chạy quyền
Administrator.


### Compression profiles and large sources (2026-10-12)

Source browsing reads only direct children. Use **Open folder** to load the next
level; the tree count and path search refer only to loaded items. Checking a
folder includes its children even if they have not been opened.

**Project** (default) skips common dependency/build/cache/log directories at any
depth, plus .log/.tmp files, and keeps .git/.env. **Everything** removes these
automatic exclusions. **Skip caches/logs** keeps dependencies and builds.
Review the editable folder-name and extension lists before creating an archive;
folders named build/dist/target can contain important output in some projects.
Tree checkboxes override name/extension defaults. Explicit unchecked paths and
Rust regex rules take priority; edit a regex to re-include a match.

Enter a profile name and use **Save profile** to persist mode, rules, and relative
selections in the existing portable/installed settings file. Load/Delete manage
these profiles. Source and output folders are chosen separately; choosing a new
source keeps the loaded relative rules and selections. Missing relative paths
are ignored so one profile can be reused across projects.

Preview and creation collect a fresh inventory and skip excluded directories
before descent. Creation can be cancelled during preparation as well as 7-Zip
execution. Preparation remains indeterminate; percentages come from 7-Zip.

## Dọn dự án để lưu trữ — 2026-10-14

Trong nhóm **Công cụ**, mở **Dọn dự án**. Chọn một dự án hoặc thư mục chứa nhiều dự án → **Quét** → chọn các mục đủ điều kiện → **Xem trước** → kiểm tra đường dẫn/dung lượng → chọn **Đưa vào Thùng rác** (mặc định) hoặc **Xóa vĩnh viễn** → nhập `RECYCLE GENERATED` hoặc `DELETE GENERATED` theo cách đã chọn → bấm dọn. Dự án không cần Git/SVN. Không mục nào được chọn sẵn; hủy quét không xóa file. Đổi thư mục, lựa chọn hoặc cách dọn sẽ hủy xem trước và chuỗi xác nhận cũ.

Chỉ nhận diện dependency/cache theo quy tắc cụ thể, có marker dự án khi cần. Không dùng danh sách loại trừ của Nén tệp làm danh sách xóa. Không tự xóa các thư mục `cache`, `dist`, `build`, `target`, `vendor`, uploads/database/backup chung. Giữ mã nguồn, .env, manifest, lockfiles và Git/SVN; cây chứa dữ liệu được bảo vệ vẫn bị chặn. Liên kết pnpm nội bộ trong node_modules được dọn như một liên kết, không quét đích. Liên kết ra ngoài node_modules, liên kết ở thư mục gốc/cha và cache có liên kết bị chặn. Kết quả chưa đầy đủ không được dùng để xóa; hãy chọn dự án nhỏ hơn hoặc xử lý quyền đọc rồi quét lại.

**Xóa vĩnh viễn, không có hoàn tác/bản sao an toàn.** Dừng build/watch trước, kiểm tra các chỉnh sửa thủ công trong dependency/cache và khả năng tải/cài lại dependency. Quét/xem trước không chứng minh mọi dependency vẫn còn trên registry. Native kiểm tra lại toàn bộ mục chọn trước khi xóa; file/marker thay đổi sẽ chặn thao tác. Nếu lỗi I/O xuất hiện trong lúc xóa, báo số file/thư mục/byte thực sự đã xóa; hãy quét lại thay vì suy đoán mọi mục đã hoàn tất.

Sau khi kiểm tra kết quả, bấm **Nén để lưu trữ** để mở module **Nén tệp / 7-Zip** với đúng thư mục vừa dọn. Bộ lọc khởi đầu là **Toàn bộ**, giữ các file còn lại; có thể sửa trước khi **Xem trước** và **Tạo kho lưu trữ**. Nút này không tự chạy nén. Package manifest và lockfiles phải được giữ để cài lại dependency; `npm ci` cần package-lock.json hoặc npm-shrinkwrap.json phù hợp ([npm Docs](https://docs.npmjs.com/cli/v11/commands/npm-ci/)).

Các nhóm hiện được nhận diện: node_modules; .parcel-cache, .vite, .turbo, .eslintcache, .stylelintcache và riêng .next/cache khi có package.json hợp lệ; __pycache__ chỉ khi toàn bộ file là bytecode và còn source .py/.pyw tương ứng. Giữ nguyên .next/standalone cùng .nuxt/.output/.svelte-kit và các build output chung. Giới hạn quét: 100.000 mục, 20.000 thư mục, 64 cấp, 2.048 candidate; chọn dự án nhỏ hơn nếu vượt giới hạn.

Trong node_modules/candidate sinh ra đã nhận diện, tên out, coverage hoặc .cache là nội dung dependency/cache và không tự gây chặn. Quy tắc giữ output dự án vẫn áp dụng bên ngoài nhóm đó. Nếu bị chặn vì .env, database, backup, Git/SVN hoặc liên kết, lý do sẽ ghi đường dẫn tương đối cụ thể để anh kiểm tra.

**Thùng rác:** dung lượng vẫn bị chiếm cho đến khi anh dọn Thùng rác. Native tạo lớp khôi phục tạm trên cùng ổ trước khi gọi Windows; nếu không tạo được (ví dụ ổ không hỗ trợ hard link), báo lỗi và giữ nguyên mục. Nếu Windows không xác nhận tái chế, khôi phục lại hoặc báo đường dẫn dữ liệu khôi phục còn giữ. Dừng build/watch trong suốt thao tác.

Hỗ trợ bộ cài pnpm có liên kết nội bộ tới .pnpm trong node_modules; giữ package.json và lockfile. Không dọn pnpm store dùng chung. Dung lượng hiển thị là kích thước nội dung file, không bảo đảm giải phóng bấy nhiêu trên ổ đĩa vì pnpm có thể dùng hard link. Workspace/link tới thư mục ngoài node_modules vẫn bị chặn.
