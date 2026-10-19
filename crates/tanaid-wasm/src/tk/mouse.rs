pub(crate) fn mouse_button_detail_from_js(button: u8) -> Option<String> {
  match button {
    // left
    0 => Some("1".into()),
    // middle
    1 => Some("2".into()),
    // right
    2 => Some("3".into()),
    // back
    3 => Some("8".into()),
    // forward
    4 => Some("9".into()),
    _ => None,
  }
}
