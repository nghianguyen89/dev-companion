import type { Language } from "../i18n";

export const appVersion = "0.2.0";

// Update this version and append an entry whenever a user-visible tool change ships.
export const releaseNotes: Record<Language, Array<{ version: string; title: string; changes: string[] }>> = {
  en: [{ version: "0.2.0", title: "Current build", changes: ["Added multi-account Codex migration with verified ZIPs, component estimates and archive management.", "Added an explicit chat replacement mode with a verified target safety ZIP; it never merges SQLite or copies auth.json.", "Clarified SourceTree configuration recovery stages, added archive management, and reduced the new-bundle password minimum to six characters.", "Unified portable Codex and personal application archives in backups; legacy SourceTree configuration ZIPs move without overwriting.", "Added the in-app Guide and About pages.", "Removed the separate legacy session backup and restore screen; guarded local session deletion remains in Conversations."] }],
  vi: [{ version: "0.2.0", title: "Bản hiện tại", changes: ["Thêm di chuyển nhiều tài khoản Codex với ZIP đã kiểm chứng, ước tính dung lượng và quản lý các bản backup.", "Thêm chế độ thay dữ liệu chat rõ ràng, tạo ZIP safety đã kiểm chứng cho máy đích; không merge SQLite hoặc chép auth.json.", "Làm rõ các bước phục hồi cấu hình SourceTree, thêm quản lý bundle và giảm mật khẩu tối thiểu của bundle mới còn 6 ký tự.", "Gộp archive Codex và bundle ứng dụng vào backups portable; ZIP cấu hình SourceTree cũ được chuyển mà không ghi đè.", "Thêm trang Hướng dẫn và Thông tin ngay trong ứng dụng.", "Loại bỏ màn hình backup/khôi phục session legacy riêng; xóa session cục bộ có bảo vệ vẫn nằm trong Phiên trò chuyện."] }],
};
