use console::Term;
use indicatif::TermLike;

pub struct PrintErr;

impl PrintErr {
    pub fn print_to(term: &Term, err: &str, msg: &str) {
        let red_text = console::Style::from_dotted_str("red");
        let err_fmt = red_text.apply_to(err);
        term.write_line(&format!("{} {}", err_fmt, msg)).unwrap();
    }

    pub fn wait_for_key(term: &Term) {
        term.write_line("Press any key to exit").unwrap();
        while !crossterm::event::read().unwrap().is_key_press() {}
    }
}