use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = "src/ui.rs";
    let mut content = fs::read_to_string(path)?;

    let mappings = vec![
        ("Ãƒâ€žÃ‚Â ÃƒÆ’Ã‚Â£ tÃƒÂ¡Ã‚ÂºÃ‚Â£i vÃƒÆ’Ã‚Â  cÃƒÆ’Ã‚Â i Interception driver. NÃƒÂ¡Ã‚ÂºÃ‚Â¿u Windows chÃƒâ€ Ã‚Â°a nhÃƒÂ¡Ã‚ÂºÃ‚Â­n ngay, hÃƒÆ’Ã‚Â£y khÃƒÂ¡Ã‚Â»Ã…Â¸i Ãƒâ€žÃ¢â‚¬ËœÃƒÂ¡Ã‚Â»Ã¢â€žÂ¢ng lÃƒÂ¡Ã‚ÂºÃ‚Â¡i mÃƒÆ’Ã‚Â¡y.", "Đã tải và cài Interception driver. Nếu Windows chưa nhận ngay, hãy khởi động lại máy."),
        ("Ãƒâ€žÃ‚Â ÃƒÆ’Ã‚Â£ gÃƒÂ¡Ã‚Â»Ã‚Â¡ Interception driver vÃƒÆ’Ã‚Â  xÃƒÆ’Ã‚Â³a bÃƒÂ¡Ã‚Â»Ã¢â€žÂ¢ cÃƒÆ’Ã‚Â i Ãƒâ€žÃ¢â‚¬ËœÃƒÆ’Ã‚Â£ tÃƒÂ¡Ã‚ÂºÃ‚Â£i. CÃƒÆ’Ã‚Â³ thÃƒÂ¡Ã‚Â»Ã†â€™ cÃƒÂ¡Ã‚ÂºÃ‚Â§n khÃƒÂ¡Ã‚Â»Ã…Â¸i Ãƒâ€žÃ¢â‚¬ËœÃƒÂ¡Ã‚Â»Ã¢â€žÂ¢ng lÃƒÂ¡Ã‚ÂºÃ‚Â¡i Windows Ãƒâ€žÃ¢â‚¬ËœÃƒÂ¡Ã‚Â»Ã†â€™ gÃƒÂ¡Ã‚Â»Ã‚Â¡ hÃƒÂ¡Ã‚ÂºÃ‚Â³n.", "Đã gỡ Interception driver và xóa bộ cài đã tải. Có thể cần khởi động lại Windows để gỡ hẳn."),
        ("Ãƒâ€žÃ‚Â ÃƒÂ¡Ã‚Â»Ã¢â‚¬Â¢i ngÃƒÆ’Ã‚Â´n ngÃƒÂ¡Ã‚Â»Ã‚Â¯", "Đổi ngôn ngữ"),
        ("Ãƒâ€žÃ‚Â ÃƒÂ¡Ã‚Â»Ã¢â‚¬Â¢i giao diÃƒÂ¡Ã‚Â»Ã¢â‚¬Â¡n sÃƒÆ’Ã‚Â¡ng / tÃƒÂ¡Ã‚Â»Ã¢â‚¬Ëœi", "Đổi giao diện sáng / tối"),
        ("Ãƒâ€žÃ‚Â ang ÃƒÂ¡Ã‚Â»Ã…Â¸ chÃƒÂ¡Ã‚ÂºÃ‚Â¿ Ãƒâ€žÃ¢â‚¬ËœÃƒÂ¡Ã‚Â»Ã¢â€žÂ¢ bÃƒÂ¡Ã‚ÂºÃ‚Â¯t phÃƒÆ’Ã‚Â­m. GiÃƒÂ¡Ã‚Â»Ã‚Â¯ combo rÃƒÂ¡Ã‚Â»Ã¢â‚¬Å“i thÃƒÂ¡Ã‚ÂºÃ‚Â£ tay Ãƒâ€žÃ¢â‚¬ËœÃƒÂ¡Ã‚Â»Ã†â€™ lÃƒâ€ Ã‚Â°u. NhÃƒÂ¡Ã‚ÂºÃ‚Â¥n Esc Ãƒâ€žÃ¢â‚¬ËœÃƒÂ¡Ã‚Â»Ã†â€™ hÃƒÂ¡Ã‚Â»Ã‚Â§y.", "Đang ở chế độ bắt phím. Giữ combo rồi thả tay để lưu. Nhấn Esc để hủy."),
        ("ChÃƒÂ¡Ã‚Â»Ã‚Â n file ÃƒÆ’Ã‚Â¢m thanh", "Chọn file âm thanh"),
        ("Ãƒâ€žÃ‚Â ang nghe thÃƒÂ¡Ã‚Â»Ã‚Â­ {title}.", "Đang nghe thử {title}."),
        ("Ãƒâ€žÃ‚Â ÃƒÆ’Ã‚Â£ dÃƒÂ¡Ã‚Â»Ã‚Â«ng nghe thÃƒÂ¡Ã‚Â»Ã‚Â­ {title}.", "Đã dừng nghe thử {title}."),
        ("ChÃƒâ€ Ã‚Â°a chÃƒÂ¡Ã‚Â»Ã‚Â n file ÃƒÆ’Ã‚Â¢m thanh.", "Chưa chọn file âm thanh."),
        ("Ãƒâ€žÃ‚Â oÃƒÂ¡Ã‚ÂºÃ‚Â¡n hiÃƒÂ¡Ã‚Â»Ã¢â‚¬Â¡n tÃƒÂ¡Ã‚ÂºÃ‚Â¡i", "Đoạn hiện tại"),
        ("ChÃƒÂ¡Ã‚Â»Ã‚Â ", "Chờ"),
        ("ÃƒÆ’Ã‚Â p cÃƒÂ¡Ã‚Â»Ã‚Â­a sÃƒÂ¡Ã‚Â»Ã¢â‚¬Â¢", "Áp cửa sổ"),
        ("BÃƒÂ¡Ã‚Â»Ã‚Â  ghim", "Bỏ ghim"),
        ("Ãƒâ€žÃ‚Â Ãƒâ€ Ã‚Â°ÃƒÂ¡Ã‚Â»Ã‚Â ng chuÃƒÂ¡Ã‚Â»Ã¢â€žÂ¢t", "Đường chuột"),
        ("Ãƒâ€žÃ‚Â ÃƒÂ¡Ã‚Â»Ã¢â€žÂ¢ nhÃƒÂ¡Ã‚ÂºÃ‚Â¡y", "Độ nhạy"),
        ("Ãƒâ€žÃ‚Â ang bÃƒÂ¡Ã‚ÂºÃ‚Â¯t...", "Đang bắt..."),
        ("Ãƒâ€žÃ‚Â ang bÃƒÂ¡Ã‚ÂºÃ‚Â¯t trigger. GiÃƒÂ¡Ã‚Â»Ã‚Â¯ rÃƒÂ¡Ã‚Â»Ã¢â‚¬Å“i nhÃƒÂ¡Ã‚ÂºÃ‚Â£ Ãƒâ€žÃ¢â‚¬ËœÃƒÂ¡Ã‚Â»Ã†â€™ lÃƒâ€ Ã‚Â°u, nhÃƒÂ¡Ã‚ÂºÃ‚Â¥n lÃƒÂ¡Ã‚ÂºÃ‚Â¡i nÃƒÆ’Ã‚Âºt bÃƒÂ¡Ã‚ÂºÃ‚Â¯t Ãƒâ€žÃ¢â‚¬ËœÃƒÂ¡Ã‚Â»Ã†â€™ hÃƒÂ¡Ã‚Â»Ã‚Â§y.", "Đang bắt trigger. Giữ rồi nhả để lưu, nhấn lại nút bắt để hủy."),
        ("Ãƒâ€žÃ‚Â ÃƒÆ’Ã‚Â£ hÃƒÂ¡Ã‚Â»Ã‚Â§y bÃƒÂ¡Ã‚ÂºÃ‚Â¯t tÃƒÂ¡Ã‚Â»Ã‚Â a Ãƒâ€žÃ¢â‚¬ËœÃƒÂ¡Ã‚Â»Ã¢â€žÂ¢ chuÃƒÂ¡Ã‚Â»Ã¢â€žÂ¢t.", "Đã hủy bắt tọa độ chuột."),
        ("KhÃƒÆ’Ã‚Â´ng tÃƒÆ’Ã‚Â¬m thÃƒÂ¡Ã‚ÂºÃ‚Â¥y step Ãƒâ€žÃ¢â‚¬ËœÃƒÂ¡Ã‚Â»Ã†â€™ bÃƒÂ¡Ã‚ÂºÃ‚Â¯t tÃƒÂ¡Ã‚Â»Ã‚Â a Ãƒâ€žÃ¢â‚¬ËœÃƒÂ¡Ã‚Â»Ã¢â€žÂ¢ chuÃƒÂ¡Ã‚Â»Ã¢â€žÂ¢t.", "Không tìm thấy step để bắt tọa độ chuột."),
        ("Ãƒâ€žÃ‚Â ÃƒÆ’Ã‚Â£ lÃƒÂ¡Ã‚ÂºÃ‚Â¥y tÃƒÂ¡Ã‚Â»Ã‚Â a Ãƒâ€žÃ¢â‚¬ËœÃƒÂ¡Ã‚Â»Ã¢â€žÂ¢ chuÃƒÂ¡Ã‚Â»Ã¢â€žÂ¢t {}, {}.", "Đã lấy tọa độ chuột {}, {}."),
        ("Ãƒâ€žÃ‚Â ang chÃƒÂ¡Ã‚Â»Ã‚Â n...", "Đang chọn..."),
        ("Ãƒâ€žÃ‚Â ÃƒÆ’Ã‚Â£ gÃƒÆ’Ã‚Â¡n hotkey bÃƒÂ¡Ã‚ÂºÃ‚Â­t/tÃƒÂ¡Ã‚ÂºÃ‚Â¯t macro.", "Đã gán hotkey bật/tắt macro."),
        ("Ã„Â ÃƒÂ£ nhÃ¡ÂºÂ­n phÃƒÂ­m: {label}. GiÃ¡Â»Â¯ thÃƒÂªm phÃƒÂ­m khÃƒÂ¡c Ã„â€˜Ã¡Â»Æ’ thÃƒÂnh combo, hoÃ¡ÂºÂ·c thÃ¡ÂºÂ£ ra Ã„â€˜Ã¡Â»Æ’ lÃ†Â°u.", "Đã nhận phím: {label}. Giữ thêm phím khác để thành combo, hoặc thả ra để lưu."),
        ("Ã„Â ÃƒÂ£ nhÃ¡ÂºÂ­n combo: {label}. ThÃ¡ÂºÂ£ ra Ã„â€˜Ã¡Â»Æ’ lÃ†Â°u.", "Đã nhận combo: {label}. Thả ra để lưu."),
        ("Ãƒâ€žÃ‚Â ÃƒÂ¡Ã‚Â»Ã¢â€žÂ¢ lÃƒÂ¡Ã‚Â»Ã¢â‚¬Â¡ch ngang", "Độ lệch ngang"),
        ("NÃƒÂ¡Ã‚ÂºÃ‚Â¿u bÃƒÂ¡Ã‚ÂºÃ‚Â­t, preset sÃƒÂ¡Ã‚ÂºÃ‚Â½ xÃƒÆ’Ã‚Â³a thanh tiÃƒÆ’Ã‚Âªu Ãƒâ€žÃ¢â‚¬ËœÃƒÂ¡Ã‚Â»Ã‚Â  trÃƒâ€ Ã‚Â°ÃƒÂ¡Ã‚Â»Ã¢â‚¬Âºc khi ÃƒÆ’Ã‚Â¡p dÃƒÂ¡Ã‚Â»Ã‚Â¥ng kÃƒÆ’Ã‚Â­ch thÃƒâ€ Ã‚Â°ÃƒÂ¡Ã‚Â»Ã¢â‚¬Âºc vÃƒÆ’Ã‚Â  vÃƒÂ¡Ã‚Â»Ã¢â‚¬Â¹ trÃƒÆ’Ã‚Â­. NÃƒÂ¡Ã‚ÂºÃ‚Â¿u tÃƒÂ¡Ã‚ÂºÃ‚Â¯t, thanh tiÃƒÆ’Ã‚Âªu Ãƒâ€žÃ¢â‚¬ËœÃƒÂ¡Ã‚Â»Ã‚Â  sÃƒÂ¡Ã‚ÂºÃ‚Â½ Ãƒâ€žÃ¢â‚¬ËœÃƒâ€ Ã‚Â°ÃƒÂ¡Ã‚Â»Ã‚Â£c giÃƒÂ¡Ã‚Â»Ã‚Â¯ hoÃƒÂ¡Ã‚ÂºÃ‚Â·c khÃƒÆ’Ã‚Â´i phÃƒÂ¡Ã‚Â»Ã‚Â¥c.", "Nếu bật, preset sẽ xóa thanh tiêu đề trước khi áp dụng kích thước và vị trí. Nếu tắt, thanh tiêu đề sẽ được giữ hoặc khôi phục."),
        ("TÃƒÆ’Ã‚Â¹y chÃƒÂ¡Ã‚Â»Ã‚Â n nÃƒÆ’Ã‚Â y khÃƒÆ’Ã‚Â´ng Ãƒâ€žÃ¢â‚¬ËœÃƒÂ¡Ã‚Â»Ã¢â‚¬Â¢i hÃƒÆ’Ã‚Â nh Ãƒâ€žÃ¢â‚¬ËœÃƒÂ¡Ã‚Â»Ã¢â€žÂ¢ng Apply bÃƒÆ’Ã‚Â¬nh thÃƒâ€ Ã‚Â°ÃƒÂ¡Ã‚Â»Ã‚Â ng hay Animated Apply. NÃƒÆ’Ã‚Â³ chÃƒÂ¡Ã‚Â»Ã¢â‚¬Â° bÃƒÂ¡Ã‚ÂºÃ‚Â­t thÃƒÆ’Ã‚Âªm mÃƒÂ¡Ã‚Â»Ã¢â€žÂ¢t phÃƒÆ’Ã‚Â­m tÃƒÂ¡Ã‚ÂºÃ‚Â¯t Ãƒâ€žÃ¢â‚¬ËœÃƒÂ¡Ã‚Â»Ã†â€™ khÃƒÆ’Ã‚Â´i phÃƒÂ¡Ã‚Â»Ã‚Â¥c thanh tiÃƒÆ’Ã‚Âªu Ãƒâ€žÃ¢â‚¬ËœÃƒÂ¡Ã‚Â»Ã‚Â  vÃƒÂ¡Ã‚Â»Ã‚Â  sau.", "Tùy chọn này không đổi hành động Apply bình thường hay Animated Apply. Nó chỉ bật thêm một phím tắt để khôi phục thanh tiêu đề về sau."),
        ("macro group ra khÃƒÂ¡Ã‚Â»Ã‚Â i nÃƒÆ’Ã‚Â³", "macro group ra khỏi nó"),
        ("ChÃƒÂ¡Ã‚Â»Ã‚Â n preset image search", "Chọn preset image search"),
        ("ChÃƒÂ¡Ã‚Â»Ã‚Â n preset hÃƒÂ¡Ã‚Â»Ã¢â€žÂ¢p cÃƒÆ’Ã‚Â´ng cÃƒÂ¡Ã‚Â»Ã‚Â¥", "Chọn preset hộp công cụ"),
        ("ThÃªm cáº§c bÆ°á»›c tiáº¿p theo bằng AI", "Thêm các bước tiếp theo bằng AI"),
        ("Thu nhÃƒÂ¡Ã‚Â»Ã‚Â  app rÃƒÂ¡Ã‚Â»Ã¢â‚¬Å“i bÃƒÂ¡Ã‚ÂºÃ‚Â¥m vÃƒÆ’Ã‚Â o bÃƒÂ¡Ã‚ÂºÃ‚Â¥t kÃƒÂ¡Ã‚Â»Ã‚Â³ vÃƒÂ¡Ã‚Â»Ã¢â‚¬Â¹ trÃƒÆ’Ã‚Â­ nÃƒÆ’Ã‚Â o trÃƒÆ’Ã‚Âªn mÃƒÆ’Ã‚Â n hÃƒÆ’Ã‚Â¬nh Ãƒâ€žÃ¢â‚¬ËœÃƒÂ¡Ã‚Â»Ã†â€™ lÃƒÂ¡Ã‚ÂºÃ‚Â¥y X/Y.", "Thu nhỏ app rồi bấm vào bất kỳ vị trí nào trên màn hình để lấy X/Y."),
        ("Di chuyÃƒÂ¡Ã‚Â»Ã†â€™n chuÃƒÂ¡Ã‚Â»Ã¢â€žÂ¢t vÃƒÂ¡Ã‚Â»Ã¢â‚¬Âºi tÃƒÂ¡Ã‚Â»Ã¢â‚¬Ëœc Ãƒâ€žÃ¢â‚¬ËœÃƒÂ¡Ã‚Â»Ã‚Â u", "Di chuyển chuột với tốc độ đầu"),
        ("DÃƒÆ’Ã‚Â¹ng thÃƒÂ¡Ã‚Â»Ã‚Â i gian hiÃƒÂ¡Ã‚Â»Ã†â€™n thÃƒÂ¡Ã‚Â»Ã¢â‚¬Â¹ riÃƒÆ’Ã‚Âªng cho step nÃƒÆ’Ã‚Â y", "Dùng thời gian hiển thị riêng cho step này"),
        ("Ghi mÃƒÂ¡Ã‚Â»Ã¢â€žÂ¢t Ãƒâ€žÃ¢â‚¬ËœÃƒâ€ Ã‚Â°ÃƒÂ¡Ã‚Â»Ã‚Â ng chuÃƒÂ¡Ã‚Â»Ã¢â€žÂ¢t Ãƒâ€žÃ¢â‚¬ËœÃƒÂ¡Ã‚Â»Ã†â€™ xem trÃƒâ€ Ã‚Â°ÃƒÂ¡Ã‚Â»Ã¢â‚¬Âºc tÃƒÂ¡Ã‚ÂºÃ‚Â¡i Ãƒâ€žÃ¢â‚¬ËœÃƒÆ’Ã‚Â¢y", "Ghi một đường chuột để xem trước tại đây"),
        ("BÃƒÂ¡Ã‚ÂºÃ‚Â¥m vÃƒÆ’Ã‚Â o Ãƒâ€žÃ¢â‚¬ËœiÃƒÂ¡Ã‚Â»Ã†â€™m muÃƒÂ¡Ã‚Â»Ã¢â‚¬Ëœn lÃƒÂ¡Ã‚ÂºÃ‚Â¥y tÃƒÂ¡Ã‚Â»Ã‚Â a Ãƒâ€žÃ¢â‚¬ËœÃƒÂ¡Ã‚Â»Ã¢â€žÂ¢ chuÃƒÂ¡Ã‚Â»Ã¢â€žÂ¢t X/Y. NhÃƒÂ¡Ã‚ÂºÃ‚Â¥n Esc Ãƒâ€žÃ¢â‚¬ËœÃƒÂ¡Ã‚Â»Ã†â€™ hÃƒÂ¡Ã‚Â»Ã‚Â§y.", "Bấm vào điểm muốn lấy tọa độ chuột X/Y. Nhấn Esc để hủy."),
        ("Ãƒâ€žÃ‚Â ang nghe lÃƒÂ¡Ã‚ÂºÃ‚Â¡i {title} tÃƒÂ¡Ã‚Â»Ã‚Â« Ãƒâ€žÃ¢â‚¬ËœÃƒÂ¡Ã‚ÂºÃ‚Â§u.", "Đang nghe lại {title} từ đầu."),
        ("MÃƒÆ’Ã‚Â u nÃƒÂ¡Ã‚Â»Ã‚Â n", "Màu nền"),
        ("Ãƒâ€žÃ‚Â ÃƒÂ¡Ã‚Â»Ã¢â€žÂ¢ mÃƒÂ¡Ã‚Â»Ã‚Â  nÃƒÂ¡Ã‚Â»Ã‚Â n", "Độ mờ nền"),
        ("NÃƒÂ¡Ã‚Â»Ã‚Â n bo gÃƒÆ’Ã‚Â³c", "Nền bo góc"),
    ];

    let mut count = 0;
    for (mangled, clean) in mappings {
        if content.contains(mangled) {
            content = content.replace(mangled, clean);
            count += 1;
        }
    }

    fs::write(path, content)?;
    println!("Replaced {} difficult string patterns successfully!", count);
    Ok(())
}
