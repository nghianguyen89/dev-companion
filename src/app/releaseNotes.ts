import type { Language } from "../i18n";

export const appVersion = "0.2.0";

// Update this version and append an entry whenever a user-visible tool change ships.
export const releaseNotes: Record<Language, Array<{ version: string; title: string; changes: string[] }>> = {
  en: [{ version: "0.2.0", title: "Current build", changes: ["Added multi-account Codex migration with verified ZIPs, component estimates and archive management.", "Added the in-app Guide and About pages.", "Removed the separate legacy session backup and restore screen; guarded local session deletion remains in Conversations."] }],
  vi: [{ version: "0.2.0", title: "Bản hiện tại", changes: ["Thêm di chuyển nhiều tài khoản Codex với ZIP đã kiểm chứng, ước tính dung lượng và quản lý các bản backup.", "Thêm trang Hướng dẫn và Thông tin ngay trong ứng dụng.", "Loại bỏ màn hình backup/khôi phục session legacy riêng; xóa session cục bộ có bảo vệ vẫn nằm trong Phiên trò chuyện."] }],
};
