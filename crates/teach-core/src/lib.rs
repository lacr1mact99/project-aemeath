//! Lõi Teach a Task (spec 001). Bề mặt điều khiển: contracts/control-surface.md.
pub mod bench;
pub mod compiler;
pub mod engine;
pub mod eval;
pub mod i18n;
pub mod model;
pub mod platform;
pub mod recorder;
pub mod runner;
pub mod scheduler;
pub mod secrets;
pub mod skill;
pub mod store;
pub mod trace;

/// FNV-1a 64-bit — một hàm cho dấu vân tay thực thi, `revision` của skill và dấu vân tay eval (data-model).
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Chuẩn hoá để so khớp: chữ thường, bỏ dấu tiếng Việt (cả dạng dựng sẵn lẫn dấu tổ hợp), `đ` → `d`,
/// gộp khoảng trắng (research R-5).
pub fn fold(s: &str) -> String {
    const MAP: &[(char, &str)] = &[
        ('a', "àáảãạăằắẳẵặâầấẩẫậ"),
        ('e', "èéẻẽẹêềếểễệ"),
        ('i', "ìíỉĩị"),
        ('o', "òóỏõọôồốổỗộơờớởỡợ"),
        ('u', "ùúủũụưừứửữự"),
        ('y', "ỳýỷỹỵ"),
        ('d', "đ"),
    ];
    let mut out = String::with_capacity(s.len());
    let mut space = false;
    for c in s.chars().flat_map(char::to_lowercase) {
        if ('\u{0300}'..='\u{036f}').contains(&c) {
            continue; // dấu tổ hợp
        }
        if c.is_whitespace() {
            space = !out.is_empty();
            continue;
        }
        if space {
            out.push(' ');
            space = false;
        }
        out.push(MAP.iter().find(|(_, set)| set.contains(c)).map_or(c, |(base, _)| *base));
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn fold_vi() {
        assert_eq!(super::fold("  Xoá  Đơn HÀNG "), "xoa don hang");
        assert_eq!(super::fold("Lưu"), "luu");
        assert_eq!(super::fold("Lu\u{031b}u"), "luu"); // dạng tổ hợp
        assert_eq!(super::fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(super::fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
    }
}
