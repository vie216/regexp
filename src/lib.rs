use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Regexp {
    tokens: Vec<Token>,
}

impl Regexp {
    pub fn new(pattern: &str) -> Result<Self, RegexpParsingError> {
        use Atom::*;
        use Quantifier::*;

        let chars: Vec<char> = pattern.chars().collect();
        let mut tokens = Vec::new();
        let mut i = 0;

        'outer: while i < chars.len() {
            let is_last_token_exact = matches!(tokens.last(), Some((_, Exact)));

            match chars[i] {
                chr if i != 0 && chars[i - 1] == '\\' => {
                    tokens.pop();
                    tokens.push((Char(chr), Exact));
                },
                '.' => tokens.push((Wildcard, Exact)),
                '(' => {
                    for j in (i..chars.len()).rev() {
                        if chars[j] == ')' && chars[j - 1] != '\\' {
                            let expr = Self::new(&pattern[(i + 1)..j])?;
                            tokens.push((Expr(expr), Exact));
                            i = j + 1;

                            continue 'outer;
                        }
                    }

                    return Err(RegexpParsingError {
                        message: format!("unclosed parenthesis at index {}", i),
                    });
                },
                '*' if is_last_token_exact => {
                    let value = tokens.pop().unwrap().0;
                    tokens.push((value, Star));
                },
                '+' if is_last_token_exact => {
                    let value = tokens.last().unwrap().0.clone();
                    tokens.push((value, Star));
                },
                '?' if is_last_token_exact => {
                    let value = tokens.pop().unwrap().0;
                    tokens.push((value, Optional));
                },
                chr => {
                    tokens.push((Char(chr), Exact));
                },
            };

            i += 1;
        }

        Ok(Self { tokens })
    }

    pub fn matches(&self, string: &str) -> bool {
        let chars: Vec<char> = string.chars().collect();
        self.start_match(&chars) == Match::Full
    }

    fn start_match(&self, chars: &[char]) -> Match {
        let mut i = 0;
        let mut consumed = 0;

        while i < self.tokens.len() {
            match &self.tokens[i] {
                (value, Quantifier::Exact) => {
                    if consumed == chars.len() {
                        return Match::Partial(consumed);
                    }

                    match value_match_len_at_index(chars, consumed, value) {
                        Match::Full => unimplemented!(),
                        Match::Partial(just_consumed) => {
                            if just_consumed == 0 {
                                return Match::Partial(consumed);
                            }

                            i += 1;
                            consumed += just_consumed;
                        },
                    }
                },
                (value, Quantifier::Star) => {
                    if consumed == chars.len() {
                        if i == self.tokens.len() - 1 {
                            return Match::Full;
                        }

                        return Match::Partial(consumed);
                    }

                    i += 1;

                    let rest = Self {
                        tokens: self.tokens[i..].to_vec(),
                    };
                    let mut j = i;

                    while j < chars.len() {
                        if rest.start_match(&chars[j..]) == Match::Full {
                            return Match::Full;
                        }

                        match value_match_len_at_index(chars, j, value) {
                            Match::Full => unimplemented!(),
                            Match::Partial(just_consumed) => {
                                if just_consumed == 0 {
                                    break;
                                }

                                consumed += just_consumed;
                            },
                        }

                        j += 1;
                    }
                },
                (value, Quantifier::Optional) => {
                    if consumed == chars.len() {
                        if i == self.tokens.len() - 1 {
                            return Match::Full;
                        }

                        return Match::Partial(consumed);
                    }

                    i += 1;

                    let rest = Self {
                        tokens: self.tokens[i..].to_vec(),
                    };

                    if rest.start_match(&chars[consumed..]) == Match::Full {
                        return Match::Full;
                    }

                    match value_match_len_at_index(chars, consumed, value) {
                        Match::Full => unimplemented!(),
                        Match::Partial(just_consumed) => consumed += just_consumed,
                    };
                },
            }
        }

        match consumed == chars.len() {
            true => Match::Full,
            false => Match::Partial(consumed),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegexpParsingError {
    pub message: String,
}

impl Display for RegexpParsingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for RegexpParsingError {}

fn value_match_len_at_index(chars: &[char], index: usize, value: &Atom) -> Match {
    match value {
        Atom::Wildcard => Match::Partial(1),
        Atom::Char(chr) => Match::Partial((chars[index] == *chr) as usize),
        Atom::Expr(expr) => expr.start_match(chars),
    }
}

type Token = (Atom, Quantifier);

#[derive(Debug, PartialEq, Eq)]
enum Match {
    Full,
    Partial(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Atom {
    Wildcard,
    Char(char),
    Expr(Regexp),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Quantifier {
    Exact,
    Star,
    Optional,
}

#[test]
fn test() {
    let expr0 = Regexp::new("ab.?c").unwrap();
    let expr1 = Regexp::new("a+b*\\.").unwrap();
    let expr2 = Regexp::new("a.*b").unwrap();

    assert!(expr0.matches("abc"));
    assert!(expr0.matches("abdc"));
    assert!(!expr0.matches("abcde"));

    assert!(expr1.matches("abbb."));
    assert!(expr1.matches("aaaa."));
    assert!(!expr1.matches("b."));
    assert!(!expr1.matches("ab!"));

    assert!(expr2.matches("asadf.b"));
    assert!(expr2.matches("ab"));
}
