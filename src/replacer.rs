use crate::query::Query;
use inflector::cases::camelcase::to_camel_case;
use inflector::cases::kebabcase::to_kebab_case;
use inflector::cases::pascalcase::to_pascal_case;
use inflector::cases::screamingsnakecase::to_screaming_snake_case;
use inflector::cases::snakecase::to_snake_case;
use inflector::cases::traincase::to_train_case;

use regex::Regex;

/// Execute a query on a line of input.
/// If there was a match, return a Replacement
///
/// Example
///
/// ```
/// use ruplacer::{Query, replace};
///
/// let input = "this is some old text";
/// let query = Query::simple("old", "new");
/// let replacement = replace(input, &query).unwrap();
/// let output = replacement.output();
/// assert_eq!(output, "this is some new text");
/// ```
pub fn replace<'a>(input: &'a str, query: &Query) -> Option<Replacement<'a>> {
    // This occurs in two steps:
    // 1/ Compute the input and output fragments - this depends
    //    on the query enum variant
    // 2/ Use the list of fragments to build the output string
    //   (this uses the same code for every query enum variant)
    let fragments = get_fragments(input, query);
    if fragments.is_empty() {
        return None;
    }
    let output = get_output(input, &fragments);
    Some(Replacement {
        fragments,
        input,
        output,
    })
}

#[derive(Debug)]
/// A replacement contains of fragments, the input string and the output string
pub struct Replacement<'a> {
    fragments: Fragments,
    input: &'a str,
    output: String,
}

impl<'a> Replacement<'a> {
    /// Return the output string
    pub fn output(&self) -> &str {
        &self.output
    }

    /// Return the input string
    pub fn input(&self) -> &str {
        self.input
    }

    pub(crate) fn num_fragments(&self) -> usize {
        self.fragments.len()
    }

    pub(crate) fn fragments(&self) -> &Fragments {
        &self.fragments
    }
}

// A list of input_fragment, output_fragment
#[derive(Debug)]
pub(crate) struct Fragments(Vec<(Fragment, Fragment)>);

impl Fragments {
    fn new() -> Self {
        Self(vec![])
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn add(
        &mut self,
        (input_index, input_text): (usize, &str),
        (output_index, output_text): (usize, &str),
    ) {
        self.0.push((
            Fragment {
                index: input_index,
                text: input_text.to_string(),
            },
            Fragment {
                index: output_index,
                text: output_text.to_string(),
            },
        ));
    }
}

impl<'a> IntoIterator for &'a Fragments {
    type Item = &'a (Fragment, Fragment);

    type IntoIter = std::slice::Iter<'a, (Fragment, Fragment)>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

/// Represent a fragment of text, similar to the data structure returned
/// by String::match_indices
#[derive(Debug)]
pub(crate) struct Fragment {
    pub(crate) index: usize,
    pub(crate) text: String,
}

trait Replacer {
    // return index of the match, input_text, output_text, or None
    fn replace(&self, buff: &str) -> Option<(usize, String, String)>;
}

struct SubstringReplacer<'a> {
    pattern: &'a str,
    replacement: &'a str,
}

impl<'a> SubstringReplacer<'a> {
    fn new(pattern: &'a str, replacement: &'a str) -> Self {
        Self {
            pattern,
            replacement,
        }
    }
}

impl<'a> Replacer for SubstringReplacer<'a> {
    fn replace(&self, buff: &str) -> Option<(usize, String, String)> {
        let index = buff.find(self.pattern)?;
        Some((
            index,
            self.pattern.to_string(),
            self.replacement.to_string(),
        ))
    }
}

struct PreserveCaseReplacer<'a> {
    items: &'a [(String, String)],
}

impl<'a> PreserveCaseReplacer<'a> {
    fn new(items: &'a [(String, String)]) -> Self {
        Self { items }
    }
}

impl<'a> Replacer for PreserveCaseReplacer<'a> {
    fn replace(&self, buff: &str) -> Option<(usize, String, String)> {
        // Note: replacing using preserve_case can get tricky
        //
        // Let's say self.items contains
        //  [("old", "new"), ("Old", "New")]
        // and that input is "
        //   "Old old = new Old();"
        //
        // Due to the algorithm used in get_output(), the fragments
        // must be in the correct order - which means we must return
        // (0, "Old", "New"), the match corresponding to the *second
        // item*), and not (4, "old","new"), the match corresponding
        // to the first item.
        //
        // Otherwise, the output would end up like this:
        //     "Old new = new New();";
        // which is not what we want!
        let mut best_index = buff.len();
        let mut best_pattern = None;
        for (i, (pattern, _)) in self.items.iter().enumerate() {
            if let Some(index) = buff.find(pattern) {
                // We found a match, but is it the best ?
                if index < best_index {
                    // Ok, record the best pattern so far:
                    best_index = index;
                    best_pattern = Some(i);
                }
            }
        }

        let best_item = &self.items[best_pattern?];
        let (pattern, replacement) = best_item;
        Some((best_index, pattern.to_string(), replacement.to_string()))
    }
}

struct RegexReplacer<'a> {
    regex: &'a Regex,
    replacement: &'a str,
}

impl<'a> RegexReplacer<'a> {
    fn new(regex: &'a Regex, replacement: &'a str) -> Self {
        Self { regex, replacement }
    }
}

impl<'a> Replacer for RegexReplacer<'a> {
    fn replace(&self, buff: &str) -> Option<(usize, String, String)> {
        let regex_match = self.regex.find(buff)?;
        let index = regex_match.start();
        let input_text = regex_match.as_str();
        let output_text = self.regex.replacen(input_text, 1, self.replacement);
        Some((index, input_text.to_string(), output_text.to_string()))
    }
}

/// Return a list of fragments for input string and output string
/// Both lists of fragments will be used for:
///    - computing the output string
///    - printing the patch
fn get_fragments(input: &str, query: &Query) -> Fragments {
    match query {
        Query::Simple(pattern, replacement) => {
            let finder = SubstringReplacer::new(pattern, replacement);
            get_fragments_with_finder(input, finder)
        }
        Query::Regex(regex, replacement) => {
            let finder = RegexReplacer::new(regex, replacement);
            get_fragments_with_finder(input, finder)
        }
        Query::PreserveCase(pattern, replacement) => {
            fn to_ada_case(input: &str) -> String {
                to_train_case(input).replace('-', "_")
            }

            let mut items = vec![];
            for function in &[
                to_ada_case,
                to_camel_case,
                to_kebab_case,
                to_pascal_case,
                to_screaming_snake_case,
                to_snake_case,
                to_train_case,
            ] {
                items.push((function(pattern), function(replacement)));
            }

            let finder = PreserveCaseReplacer::new(&items);
            get_fragments_with_finder(input, finder)
        }
    }
}

fn get_fragments_with_finder(input: &str, finder: impl Replacer) -> Fragments {
    // Algorithm: call finder.find(). If it matches, bump input_index and output_text
    // using the length of the input text and the length of the output text respectively
    // Truncate the input string at each step to keep finding successive matches:
    //
    // step | buf                      | input_fragment | output_fragment
    // -----|--------------------------|----------------|----------
    //    0 | "my tea is the best tea" |     (3, "tea") |  (3, "coffee")
    //    1 | "  is the best tea"      |    (19, "tea") | (22, "coffee")
    //    2 | " !"                     |      n/a       | n/a
    //
    let mut fragments = Fragments::new();
    let mut input_index = 0;
    let mut output_index = 0;
    while let Some(res) = finder.replace(&input[input_index..]) {
        let (index, input_text, output_text) = res;
        input_index += index;
        output_index += index;
        fragments.add((input_index, &input_text), (output_index, &output_text));
        input_index += input_text.len();
        output_index += output_text.len();
    }

    fragments
}

fn get_output(input: &str, fragments: &Fragments) -> String {
    let mut current_index = 0;
    let mut output = String::new();
    for (input_fragment, output_fragment) in fragments.into_iter() {
        let Fragment {
            text: input_text,
            index: input_index,
        } = input_fragment;

        let Fragment {
            text: output_text, ..
        } = output_fragment;

        output.push_str(&input[current_index..*input_index]);
        output.push_str(output_text);
        current_index = input_index + input_text.len();
    }
    output.push_str(&input[current_index..]);
    output
}

#[cfg(test)]
mod tests {

    use crate::Console;

    use super::*;
    use regex::Regex;

    #[test]
    fn test_simple_1() {
        let input = "Mon thé c'est le meilleur des thés !";
        let pattern = "thé";
        let replacement = "café";
        let query = Query::simple(pattern, replacement);
        let replacement = replace(input, &query).unwrap();
        assert_eq!(
            replacement.output(),
            "Mon café c'est le meilleur des cafés !"
        );
    }

    #[test]
    fn test_simple_2() {
        let input = "old old old";
        let pattern = "old";
        let replacement = "new";
        let query = Query::simple(pattern, replacement);
        let replacement = replace(input, &query).unwrap();
        assert_eq!(replacement.output(), "new new new");
    }

    #[test]
    fn test_display_patch() {
        // Note: no assertion there. The test is here so it's easy
        // to tweak ruplacer's output running `cargo test -- --nocapture`
        let input = "Top: old is nice";
        let pattern = "old";
        let replacement = "new";
        let query = Query::simple(pattern, replacement);
        let replacement = replace(input, &query).unwrap();
        let console = Console::new();
        console.print_replacement("foo.txt:3 ", &replacement);
    }

    #[test]
    fn test_preserve_case() {
        let input = "let foo_bar = FooBar::new();";
        let pattern = "foo_bar";
        let replacement = "spam_eggs";
        let query = Query::preserve_case(pattern, replacement);
        let replacement = replace(input, &query).unwrap();
        assert_eq!(replacement.output(), "let spam_eggs = SpamEggs::new();");
    }

    #[test]
    fn test_regex_with_substitutions() {
        let input = "first, second";
        let regex = Regex::new(r"(\w+), (\w+)").unwrap();
        // ensure ${1}text syntax works as expected
        let query = Query::regex(regex, r"$2 ${1}third");
        let replacement = replace(input, &query).unwrap();
        assert_eq!(replacement.output(), "second firstthird");
    }

    #[test]
    fn test_simple_regex() {
        let input = "old is old";
        let regex = Regex::new("old").unwrap();
        let query = Query::regex(regex, "new");
        let replacement = replace(input, &query).unwrap();
        assert_eq!(replacement.output(), "new is new");
    }
}
