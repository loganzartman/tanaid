use tanaid::eval::{self, EvalContext};
use tanaid::eval_error::EvalError;
use tanaid::parser;

use crate::Tk;
use crate::canvas::Rect;
use crate::color::Color;

fn interpreter() -> (EvalContext, Tk) {
  let mut context = EvalContext::new();
  let tk = Tk::new();
  tk.register_commands(&mut context);
  (context, tk)
}

fn eval_source(context: &mut EvalContext, src: &str) -> Result<String, EvalError> {
  let script = parser::parse(src).expect("parses");
  Ok(eval::eval(&script, context)?.repr_str()?.to_string())
}

fn eval_ok(context: &mut EvalContext, src: &str) -> String {
  eval_source(context, src).expect("evaluates")
}

#[test]
fn creates_and_packs_a_canvas() {
  let (mut context, tk) = interpreter();

  assert_eq!(
    eval_ok(
      &mut context,
      "canvas .c -width 320 -height 240 -background black"
    ),
    ".c"
  );
  assert!(!tk.has_window(), "a canvas isn't shown until it is packed");

  eval_ok(&mut context, "pack .c");
  let canvas = tk.mapped_canvas().expect("mapped");

  assert_eq!((canvas.width, canvas.height), (320, 240));
  assert_eq!(canvas.background, Color::rgb(0x000000));
}

#[test]
fn rejects_bad_canvas_arguments() {
  let (mut context, _tk) = interpreter();

  assert!(eval_source(&mut context, "canvas c").is_err(), "path name");
  assert!(
    eval_source(&mut context, "canvas .c -fill red").is_err(),
    "unsupported option"
  );
  assert!(
    eval_source(&mut context, "canvas .c -background chartreuse").is_err(),
    "unknown color"
  );
  assert!(
    eval_source(&mut context, "pack .nothing").is_err(),
    "unknown widget"
  );
}

#[test]
fn creates_rectangles_with_ids_from_one() {
  let (mut context, tk) = interpreter();
  eval_ok(&mut context, "canvas .c; pack .c");

  assert_eq!(eval_ok(&mut context, ".c create rectangle 0 0 10 10"), "1");
  assert_eq!(
    eval_ok(&mut context, ".c create rectangle 5 5 20 20 -fill red"),
    "2"
  );

  let canvas = tk.mapped_canvas().expect("mapped");
  assert_eq!(canvas.items().len(), 2);
  assert_eq!(
    canvas.item(2).expect("second item").coords(),
    Rect {
      x1: 5.,
      y1: 5.,
      x2: 20.,
      y2: 20.
    }
  );
}

#[test]
fn moves_and_sets_coordinates() {
  let (mut context, _tk) = interpreter();
  eval_ok(&mut context, "canvas .c; pack .c");
  eval_ok(&mut context, "set box [.c create rectangle 10 20 30 40]");

  eval_ok(&mut context, ".c move $box 5 -5");
  assert_eq!(eval_ok(&mut context, ".c coords $box"), "15 15 35 35");

  eval_ok(&mut context, ".c coords $box 0 0 1 2");
  assert_eq!(eval_ok(&mut context, ".c coords $box"), "0 0 1 2");
}

#[test]
fn configures_and_deletes_items() {
  let (mut context, tk) = interpreter();
  eval_ok(&mut context, "canvas .c; pack .c");
  eval_ok(&mut context, ".c create rectangle 0 0 10 10 -fill red");
  eval_ok(&mut context, ".c create rectangle 0 0 10 10");

  eval_ok(&mut context, ".c itemconfigure 1 -fill #0000ff");
  assert!(matches!(
    tk.mapped_canvas().expect("mapped").item(1).expect("item").shape,
    crate::Shape::Rectangle {
      fill: Some(fill), ..
    } if fill == Color::rgb(0x0000ff)
  ));

  eval_ok(&mut context, ".c delete 1");
  assert_eq!(tk.mapped_canvas().expect("mapped").items().len(), 1);

  eval_ok(&mut context, ".c delete all");
  assert!(tk.mapped_canvas().expect("mapped").items().is_empty());
}

#[test]
fn ignores_tags_that_match_nothing() {
  let (mut context, _tk) = interpreter();
  eval_ok(&mut context, "canvas .c; pack .c");

  // as in Tk, an id that no longer exists is not an error to move or delete
  eval_ok(&mut context, ".c move 42 1 1");
  eval_ok(&mut context, ".c delete 42");
  assert!(
    eval_source(&mut context, ".c coords 42").is_err(),
    "but there are no coordinates to report"
  );
}

#[test]
fn rejects_unsupported_drawing() {
  let (mut context, _tk) = interpreter();
  eval_ok(&mut context, "canvas .c; pack .c");

  assert!(
    eval_source(&mut context, ".c create oval 0 0 10 10").is_err(),
    "unsupported item type"
  );
  assert!(
    eval_source(&mut context, ".c create rectangle 0 0 10 10 -outline red").is_err(),
    "unsupported item option"
  );
  assert!(
    eval_source(&mut context, ".c scale all 0 0 2 2").is_err(),
    "unsupported subcommand"
  );
}

#[test]
fn tracks_whether_the_widgets_changed() {
  let (mut context, tk) = interpreter();
  eval_ok(&mut context, "canvas .c; pack .c");
  assert!(tk.take_dirty());
  assert!(!tk.take_dirty(), "dirtiness is consumed");

  eval_ok(&mut context, ".c create rectangle 0 0 10 10");
  assert!(tk.take_dirty());

  eval_ok(&mut context, ".c coords 1");
  assert!(!tk.take_dirty(), "a query changes nothing");

  eval_ok(&mut context, ".c move 1 1 1");
  assert!(tk.take_dirty());
}
