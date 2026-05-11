use pest::Parser;

#[derive(Parser)]
#[grammar = "grammar_test/minimal.pest"]
struct MinimalGrammar;

#[test]
fn test_minimal_between() {
    let input = "(between 100 200)";
    let result = MinimalGrammar::parse(pest::Rule::query, input);
    match result {
        Ok(_) => println!("SUCCESS: {:?}", result),
        Err(e) => {
            println!("FAILED: {}", e);
            panic!("Parse failed: {}", e);
        }
    }
}
