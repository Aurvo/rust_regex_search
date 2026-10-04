use std::str::Chars;
use std::vec::Vec;

pub fn test_sequence(st: &str, sequence: &SequenceExpr) -> bool {
    let mut cur_slice = st;
    let mut match_data_opt: Option<AtomMatchData>;
    let mut char_indices_iterator = st.char_indices();
    char_indices_iterator.next();
    loop {
        match_data_opt = sequence.extract_starting_match(&mut cur_slice.chars(), None);
        if match_data_opt.is_some() { return true; }
        let Some((total_bytes_taken, _)) = char_indices_iterator.next() else {break;};
        cur_slice = &st[total_bytes_taken..];
    }
    false
}

pub fn form_expression_tree(s: &str) -> Result<SequenceExpr, String> {
    let mut sequences = vec![
        SequenceExpr {exprs: Vec::new()}
    ];
    let mut exprs = &mut sequences[0].exprs;
    let mut union_mamager: Option<UnionExprManager> = None;
    let mut is_escaping = false;
    for ch in s.chars() {
        if is_escaping {
            if let Some(um) = &mut union_mamager {
                um.add_char(ch)?;
            } else {
                exprs.push(Box::new(CharExp (ch)));
            }
            is_escaping = false;
        } else if let Some(um) = &mut union_mamager {
            if ch == '\\' {
                is_escaping = true;
            } else if ch == '-' {
                um.await_range_end();
            } else if ch == ']' {
                exprs.push(Box::new(um.clone().finish()));
                union_mamager = None;
            } else {
                um.add_char(ch)?;
            }
        } else if ch == '.' {
            exprs.push(Box::new(Wildcard));
        } else if ch == '*' {
            let Some(expr) = exprs.pop() else {
                return Err(String::from("'*' at beginning of pattern"));
            };
            exprs.push(Box::new(RepetitionExpr {expr, min_matches: 0}));
        } else if ch == '+' {
            let Some(expr) = exprs.pop() else {
                return Err(String::from("'+' at beginning of pattern"));
            };
            exprs.push(Box::new(RepetitionExpr {expr, min_matches: 1}));
        } else if ch == '(' {
            sequences.push(SequenceExpr { exprs: Vec::new() });
            let new_exprs_index = sequences.len() - 1;
            exprs = &mut sequences[new_exprs_index].exprs;
        } else if ch == ')' {
            let (Some(comlpeted_sequence), Some(prev_sequence)) =
                (sequences.pop(), sequences.last_mut())
            else {
                return Err(String::from("found ')' with no matching '('"));
            };
            exprs = &mut prev_sequence.exprs;
            exprs.push(Box::new(comlpeted_sequence));
        } else if ch == '[' {
            union_mamager = Some(UnionExprManager::new(UnionExpr { chars: String::new(), is_negated: false }));
        } else if ch == '\\' {
            is_escaping = true;
        } else {
            exprs.push(Box::new(CharExp (ch)));
        }
    }
    Ok(sequences.pop().expect("should be the first sequence in 'sequences'"))
}

struct AtomMatchData {
    num_chars: usize,
    can_shrink: bool,
}

trait Expression {
    fn extract_starting_match(&self, str_iterator: &mut Chars, char_limit: Option<usize>) -> Option<AtomMatchData>;
    fn needs_own_iterator(&self) -> bool {false}
}

// Character Expression

struct CharExp (char);

impl Expression for CharExp {
    fn extract_starting_match(&self, str_iterator: &mut Chars, _: Option<usize>) -> Option<AtomMatchData> {
        if let Some(ch) = str_iterator.next() {
            if ch == self.0 {
                return Some(AtomMatchData {
                    num_chars: 1,
                    can_shrink: false
                })
            }
        }
        None
    }
}

// Wildcard Expression

struct Wildcard;

impl Expression for Wildcard {
    fn extract_starting_match(&self, str_iterator: &mut Chars, _: Option<usize>) -> Option<AtomMatchData> {
        if str_iterator.next().is_some() {
            return Some(AtomMatchData {
                num_chars: 1,
                can_shrink: false
            })
        }
        None
    }
}

// Repetition Expression

struct RepetitionExpr {
    expr: Box<dyn Expression>,
    min_matches: usize
}

impl Expression for RepetitionExpr {
    fn extract_starting_match(&self, str_iterator: &mut Chars, char_limit: Option<usize>) -> Option<AtomMatchData> {
        let mut num_chars: usize = 0;
        let mut num_matches: usize = 0;
        let char_limit_val: usize = char_limit.unwrap_or(0);
        let mut match_data_opt: Option<AtomMatchData>;
        let shld_provide_own_iterator = self.expr.needs_own_iterator();
        loop {
            if char_limit.is_some() && num_chars >= char_limit_val {break;}
            if shld_provide_own_iterator {
                match_data_opt = self.expr.extract_starting_match(&mut str_iterator.clone(), None)
            } else {
                match_data_opt = self.expr.extract_starting_match(str_iterator, None)
            }
            let Some(match_data) = match_data_opt else { break };
            num_chars += match_data.num_chars;
            num_matches += 1;
            if shld_provide_own_iterator {
                if match_data.num_chars == 0 {
                    // at this point, we konw that further iterations
                    // will always yeild Some(match_data) with num_chars = 0,
                    // resulting in an infinite loop if not handled.
                    if num_matches < self.min_matches {
                        // We know infinitely looping would eventually set
                        // num_matches = self.min_matches. No need to go
                        //through the motions.
                        num_matches = self.min_matches;
                    }
                    break
                }
                str_iterator.nth(match_data.num_chars - 1);
            }
        }
        if num_matches >= self.min_matches {
            return Some(AtomMatchData {
                num_chars: num_chars,
                can_shrink: num_chars > self.min_matches
            })
        }
        None
    }

    fn needs_own_iterator(&self) -> bool {true}
}

// Union Expression

#[derive(Clone)]
struct UnionExprManager {
    cached_char: Option<char>,
    is_awaiting_range: bool,
    union_expr: UnionExpr
}

impl UnionExprManager {
    pub fn new(union_expr: UnionExpr) -> UnionExprManager {
        UnionExprManager {
            cached_char: None,
            is_awaiting_range: false,
            union_expr
        }
    }

    /**
     * Adds a char to the managed UnionExpr. Handles complications involved with
     * ranges of characters like a-z or 0-9.
     */
    pub fn add_char(&mut self, ch: char) -> Result<(), String> {
        if self.is_awaiting_range {
            let Some(cached_char) = self.cached_char else {
                return Err(String::from("malformed character range in [] expression"));
            };
            self.union_expr.add_range(&cached_char, &ch);
            self.cached_char = None;
            self.is_awaiting_range = false;
            return Ok(());
        }
        if let Some(cached_char) = self.cached_char {
            self.union_expr.add_char(cached_char);
        }
        self.cached_char = Some(ch);
        Ok(())
    }

    /**
     * Informs the manager that the next char belongs to the second in a range
     */
    pub fn await_range_end(&mut self) {
        self.is_awaiting_range = true;
    }

    /**
     * Flushes pending changes and consumes the UnionManager
     */
    pub fn finish(mut self) -> UnionExpr {
        if let Some(cached_char) = self.cached_char {
            self.union_expr.add_char(cached_char);
        }
        return self.union_expr;
    }
}

#[derive(Clone)]
struct UnionExpr {
    chars: String,
    is_negated: bool
}

impl UnionExpr {
    fn add_char(&mut self, ch: char) {
        self.chars.push(ch);
    }

    fn add_range(&mut self, ch1: &char, ch2: &char) {
        let range = if ch1 < ch2 { *ch1..=*ch2 } else { *ch2..=*ch1 };
        for ch in range {
            self.chars.push(ch);
        }
    }
}

impl Expression for UnionExpr {
    fn extract_starting_match(&self, str_iterator: &mut Chars, _: Option<usize>) -> Option<AtomMatchData> {
        let ch = str_iterator.next()?;
        let mut does_match = self.chars.contains(ch);
        if self.is_negated {does_match = !does_match}
        if does_match {
            return Some(AtomMatchData { num_chars: 1, can_shrink: false })
        }
        None
    }
}

// Sequence Expression

struct SeqHistoryRecord {
    num_chars: usize,
    total_num_chars: usize,
    can_shrink: bool
}

pub struct SequenceExpr {
    exprs: Vec<Box<dyn Expression>>
}

impl Expression for SequenceExpr {
    fn extract_starting_match(&self, str_iterator: &mut Chars, char_limit: Option<usize>) -> Option<AtomMatchData> {
        let exprs = &self.exprs;
        let mut str_iter_clone = str_iterator.clone();
        let num_exprs: usize = exprs.len();
        let mut total_num_chars: usize = 0;
        let mut index: usize = 0;
        let mut expr_box: &Box<dyn Expression>;
        let mut history_vector: Vec<SeqHistoryRecord> = Vec::new();
        let mut shrinkable_indices: Vec<usize> = Vec::new();
        let mut cached_char_limit: Option<usize> = None;
        let mut optnl_match: Option<AtomMatchData>;
        let mut shld_provide_seperate_iterator: bool;
        loop {
            macro_rules! try_backtracking_to_prev_shrinkable_index_and_continue_loop {
                () => {
                    index = shrinkable_indices.pop()?;
                    cached_char_limit = Some(history_vector[index].num_chars - 1);
                    history_vector.truncate(index);
                    str_iter_clone = str_iterator.clone();
                    if let Some(record) = history_vector.last() {
                        if record.total_num_chars > 0 {
                            str_iter_clone.nth(record.total_num_chars - 1);
                        }
                    }                        
                    continue;
                };
            }
            if index >= num_exprs {
                break;
            }
            expr_box = &exprs[index];
            shld_provide_seperate_iterator = expr_box.needs_own_iterator();
            if shld_provide_seperate_iterator {
                optnl_match = expr_box.extract_starting_match(&mut str_iter_clone.clone(), cached_char_limit.or(char_limit));
            } else {
                optnl_match = expr_box.extract_starting_match(&mut str_iter_clone, cached_char_limit.or(char_limit));
            }
            if cached_char_limit.is_some() { cached_char_limit = None; }
            let Some(match_data) = optnl_match else {
                try_backtracking_to_prev_shrinkable_index_and_continue_loop!();
            };
            total_num_chars += match_data.num_chars;
            if shld_provide_seperate_iterator && match_data.num_chars > 0 {
                str_iter_clone.nth(match_data.num_chars - 1);
            }
            if match_data.can_shrink {
                if shrinkable_indices.last().unwrap_or(&(index + 1)) != &index {
                    shrinkable_indices.push(index);
                }
            } else if shrinkable_indices.last().unwrap_or(&(index + 1)) == &index {
                shrinkable_indices.pop();
            }
            if char_limit.unwrap_or(total_num_chars) < total_num_chars {
                try_backtracking_to_prev_shrinkable_index_and_continue_loop!();
            }
            history_vector.push(SeqHistoryRecord {
                num_chars: match_data.num_chars,
                total_num_chars,
                can_shrink: match_data.can_shrink
            });
            index += 1;
        }
        Some(AtomMatchData {
            num_chars: total_num_chars,
            can_shrink: history_vector
                .iter().any(|record| {record.can_shrink})
        })
    }

    fn needs_own_iterator(&self) -> bool {true}
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn test_regex(st: &str, regex: &str) -> Result<bool, String> {
        let expression_tree = form_expression_tree(regex)?;
        return Ok(test_sequence(&st, &expression_tree));
    }

    fn assert_matches(st: &str, regex: &str, should_match: bool) {
        let result = test_regex(st, regex);
        let Ok(does_match) = result else {
            assert!(false, "{}", result.unwrap_err());
            return;
        };
        
        if should_match {
            assert!(does_match, "'{}' failed to match '{}'", st, regex);
        } else {
            assert!(!does_match, "'{}' matches '{}' when it should NOT", st, regex);
        }
    }

    macro_rules! do_test {
        ($func:ident, $st:expr, $regex:expr, $shld_pass:expr) => {
            #[test]
            fn $func() {
                assert_matches($st, $regex, $shld_pass);
            }
        };
    }

    do_test!(a, "abcd", "cd", true);
    do_test!(b, "yeeeeeees", "nope", false);
    do_test!(c, "all is good", "i..g", true);
    do_test!(d, "greaaat", "grea*t", true);
    do_test!(e, "greaaat", "grea*at", true);
    do_test!(f, "doe", "dog*e", true);
    do_test!(g, "dogg", "dog+", true);
    do_test!(h, "do", "dog+", false);
    do_test!(i, "byebyebye", "(bye)*", true);
    do_test!(j, "", "(bye)*", true);
    do_test!(k, "genz", "gen[xyz]", true);
    do_test!(l, "gena", "gen[xyz]", false);
    do_test!(m, "games", "g[eam]+s", true);
    do_test!(n, "1 and a 2 or move to 3", "1[a-z ]+2[a-z ]*3", true);
    do_test!(o, "something", ".(.*)+", true);
    do_test!(p, "a+[]-", "a\\+[\\[\\-\\]]+", true);
}