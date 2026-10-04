import { PageHeader } from "../../components/PageHeader";
import { useTranslation, type Language } from "../../i18n";

type GuideItem = { title: string; description: string; steps: string[] };

const guides: Record<Language, { description: string; items: GuideItem[] }> = {
  en: {
    description: "Short, practical instructions for every tool in this application.",
    items: [
      { title: "Dashboard", description: "See the local tool summary and backup inventory.", steps: ["Use Refresh to reread file metadata.", "The dashboard does not open or change backup contents."] },
      { title: "File Transfer", description: "Copy selected local files or folders with Windows Robocopy.", steps: ["Choose source and destination, then select only the direct children to copy.", "Review Analyze first; Mirror can delete destination-only files and needs confirmation."] },
      { title: "File Compression", description: "Create a 7-Zip archive from one reviewed folder.", steps: ["Choose the source, inspect the read-only tree and set exclusions.", "Choose Fast or Strong, then create a new archive without overwriting an existing one."] },
      { title: "Codex environments", description: "Keep separate local CODEX_HOME environments for Codex.", steps: ["Add the environment metadata and use its launcher or login action.", "Removing an entry keeps its files and official Codex credentials untouched."] },
      { title: "Move Codex accounts", description: "Move supported local .codex and .codex-* data to another Windows profile.", steps: ["Close Codex, choose detected accounts and the data groups to include, then create the verified ZIP.", "On the new machine inspect the ZIP, type RESTORE, and sign in again. Existing files are kept."] },
      { title: "Conversations", description: "Browse supported legacy local session metadata.", steps: ["Refresh to list local metadata only; chat contents are never shown.", "To delete selected local legacy files, preview them and type DELETE; a verified safety ZIP is made first."] },
      { title: "Skills", description: "Manage supported local custom Codex skills.", steps: ["Import or export through a new folder only.", "Restart Codex if it is running after changing skills."] },
      { title: "Pets", description: "Install or remove supported local Codex pets.", steps: ["Install a validated v2 pet into a new folder.", "Type REMOVE to permanently remove the selected pet."] },
      { title: "Environment backup", description: "Create or inspect the older environment-v2 recovery archive.", steps: ["Review the included files before creating the archive.", "Recovery creates missing files only; it does not prove Desktop chat migration works."] },
      { title: "Clean temporary data", description: "Remove only the reviewed remote plugin catalog cache.", steps: ["Run Scan and review the eligible cache files.", "Type CLEAN to remove only the selected scanned files."] },
      { title: "Beyond Compare", description: "Package one user-exported .bcpkg file.", steps: ["Export settings in Beyond Compare, then select the resulting package.", "Treat it as sensitive and import the staged copy manually in Beyond Compare."] },
      { title: "SourceTree", description: "Back up selected local SourceTree data.", steps: ["Close SourceTree and create the appropriate bookmark or encrypted configuration bundle.", "Inspect and restore to Companion staging, then review and place or import manually."] },
      { title: "XAMPP files", description: "Back up chosen htdocs projects and reviewed configuration files.", steps: ["Stop XAMPP-related processes, select direct htdocs projects and review the preview.", "Recovery stages files for manual review; it never writes directly into XAMPP."] },
      { title: "Diagnostics", description: "Read local status and paths without changing them.", steps: ["Use Refresh after changing a local installation or configuration."] },
      { title: "Settings", description: "Change local application preferences.", steps: ["Choose language, theme, log level and portable options, then save settings."] },
    ],
  },
  vi: {
    description: "Hướng dẫn ngắn, thực tế cho từng chức năng trong ứng dụng.",
    items: [
      { title: "Bảng điều khiển", description: "Xem tóm tắt công cụ cục bộ và inventory backup.", steps: ["Dùng Làm mới để đọc lại metadata file.", "Trang này không mở hoặc thay đổi nội dung backup."] },
      { title: "Chuyển tệp", description: "Chép file/thư mục cục bộ đã chọn bằng Windows Robocopy.", steps: ["Chọn nguồn, đích và chỉ tick các mục cùng cấp cần chép.", "Xem Analyze trước; Mirror có thể xóa file chỉ có ở đích và luôn cần xác nhận."] },
      { title: "Nén tệp", description: "Tạo archive 7-Zip từ một thư mục đã xem xét.", steps: ["Chọn nguồn, xem cây file chỉ đọc và đặt loại trừ.", "Chọn Nhanh hoặc Mạnh, sau đó tạo archive mới mà không ghi đè file có sẵn."] },
      { title: "Môi trường Codex", description: "Tách các môi trường CODEX_HOME cục bộ cho Codex.", steps: ["Thêm metadata môi trường rồi dùng launcher hoặc thao tác đăng nhập.", "Xóa một mục chỉ xóa metadata; file và credential Codex chính thức vẫn giữ nguyên."] },
      { title: "Di chuyển tài khoản Codex", description: "Chuyển dữ liệu .codex và .codex-* cục bộ được hỗ trợ sang Windows profile khác.", steps: ["Đóng Codex, chọn tài khoản phát hiện được và nhóm dữ liệu cần gồm, rồi tạo ZIP đã kiểm chứng.", "Ở máy mới, kiểm tra ZIP, nhập RESTORE rồi đăng nhập lại. File đích đang có luôn được giữ."] },
      { title: "Phiên trò chuyện", description: "Duyệt metadata session legacy cục bộ được hỗ trợ.", steps: ["Làm mới để xem metadata cục bộ; nội dung chat không bao giờ hiển thị.", "Muốn xóa file legacy đã chọn: xem trước, nhập DELETE; safety ZIP đã kiểm chứng được tạo trước."] },
      { title: "Kỹ năng", description: "Quản lý custom skill Codex cục bộ được hỗ trợ.", steps: ["Nhập hoặc xuất luôn vào thư mục mới, không ghi đè.", "Khởi động lại Codex nếu nó đang chạy sau khi đổi skill."] },
      { title: "Thú cưng", description: "Cài hoặc gỡ pet Codex cục bộ được hỗ trợ.", steps: ["Cài pet v2 hợp lệ vào thư mục mới.", "Nhập REMOVE để gỡ vĩnh viễn pet đã chọn."] },
      { title: "Backup môi trường", description: "Tạo hoặc kiểm tra archive khôi phục environment-v2 cũ.", steps: ["Xem lại file được gồm trước khi tạo archive.", "Khôi phục chỉ tạo file còn thiếu; không chứng minh được chat Desktop đã hoạt động sau migration."] },
      { title: "Dọn dữ liệu tạm", description: "Chỉ dọn cache catalog plugin từ xa đã được kiểm tra.", steps: ["Chạy Quét và xem lại file cache đủ điều kiện.", "Nhập CLEAN để chỉ xóa các file đã quét và đang được chọn."] },
      { title: "Beyond Compare", description: "Đóng gói một file .bcpkg do người dùng tự xuất.", steps: ["Xuất settings trong Beyond Compare, rồi chọn file package tạo ra.", "Coi đó là dữ liệu nhạy cảm và tự import bản staging trong Beyond Compare."] },
      { title: "SourceTree", description: "Backup dữ liệu SourceTree cục bộ đã chọn.", steps: ["Đóng SourceTree rồi tạo bundle bookmark hoặc bundle cấu hình mã hóa phù hợp.", "Kiểm tra và khôi phục vào staging của Companion, sau đó tự xem xét và đặt/import."] },
      { title: "Tệp XAMPP", description: "Backup project htdocs đã chọn và các file cấu hình được duyệt.", steps: ["Dừng các process liên quan XAMPP, chọn project trực tiếp trong htdocs và xem trước.", "Khôi phục chỉ staging để xem xét thủ công; không bao giờ ghi thẳng vào XAMPP."] },
      { title: "Chẩn đoán", description: "Đọc trạng thái và đường dẫn cục bộ, không thay đổi dữ liệu.", steps: ["Dùng Làm mới sau khi thay đổi cài đặt hoặc cài phần mềm cục bộ."] },
      { title: "Cài đặt", description: "Đổi tùy chọn cục bộ của ứng dụng.", steps: ["Chọn ngôn ngữ, giao diện, mức log và portable mode, rồi lưu cài đặt."] },
    ],
  },
};

export function GuidePage() {
  const { language, t } = useTranslation();
  const guide = guides[language];
  return <><PageHeader title={t("guide.title")} description={guide.description} /><section className="guide-grid">{guide.items.map((item) => <article key={item.title} className="guide-card"><h2>{item.title}</h2><p>{item.description}</p><ol>{item.steps.map((step) => <li key={step}>{step}</li>)}</ol></article>)}</section></>;
}
