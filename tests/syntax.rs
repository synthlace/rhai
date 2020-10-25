use rhai::{Engine, EvalAltResult, ParseError, ParseErrorType, Position, INT};

#[test]
fn test_custom_syntax() -> Result<(), Box<EvalAltResult>> {
    let mut engine = Engine::new();

    engine.consume("while false {}")?;

    // Disable 'while' and make sure it still works with custom syntax
    engine.disable_symbol("while");
    assert!(matches!(
        *engine.compile("while false {}").expect_err("should error").0,
        ParseErrorType::Reserved(err) if err == "while"
    ));
    assert!(matches!(
        *engine.compile("let while = 0").expect_err("should error").0,
        ParseErrorType::Reserved(err) if err == "while"
    ));

    engine.register_custom_syntax(
        &[
            "exec", "|", "$ident$", "|", "->", "$block$", "while", "$expr$",
        ],
        1,
        |context, inputs| {
            let var_name = inputs[0].get_variable_name().unwrap().to_string();
            let stmt = inputs.get(1).unwrap();
            let condition = inputs.get(2).unwrap();

            context.scope.push(var_name, 0 as INT);

            loop {
                context.eval_expression_tree(stmt)?;

                let stop = !context
                    .eval_expression_tree(condition)?
                    .as_bool()
                    .map_err(|err| {
                        Box::new(EvalAltResult::ErrorMismatchDataType(
                            "bool".to_string(),
                            err.to_string(),
                            condition.position(),
                        ))
                    })?;

                if stop {
                    break;
                }
            }

            Ok(().into())
        },
    )?;

    // 'while' is now a custom keyword so this it can no longer be a variable
    engine.consume("let while = 0").expect_err("should error");

    assert_eq!(
        engine.eval::<INT>(
            r"
                exec |x| -> { x += 1 } while x < 42;
                x
            "
        )?,
        42
    );

    // The first symbol must be an identifier
    assert_eq!(
        *engine
            .register_custom_syntax(&["!"], 0, |_, _| Ok(().into()))
            .expect_err("should error")
            .0,
        ParseErrorType::BadInput(
            "Improper symbol for custom syntax at position #1: '!'".to_string()
        )
    );

    Ok(())
}

#[test]
fn test_custom_syntax_raw() -> Result<(), Box<EvalAltResult>> {
    let mut engine = Engine::new();

    engine.register_custom_syntax_raw(
        "hello",
        |stream| match stream.len() {
            0 => unreachable!(),
            1 => Ok(Some("$ident$".into())),
            2 => match stream[1].as_str() {
                "world" | "kitty" => Ok(None),
                s => Err(ParseError(
                    Box::new(ParseErrorType::BadInput(s.to_string())),
                    Position::none(),
                )),
            },
            _ => unreachable!(),
        },
        0,
        |_, inputs| {
            Ok(match inputs[0].get_variable_name().unwrap() {
                "world" => 123 as INT,
                "kitty" => 42 as INT,
                _ => unreachable!(),
            }
            .into())
        },
    );

    assert_eq!(engine.eval::<INT>("hello world")?, 123);
    assert_eq!(engine.eval::<INT>("hello kitty")?, 42);
    assert_eq!(
        *engine.compile("hello hey").expect_err("should error").0,
        ParseErrorType::BadInput("hey".to_string())
    );

    Ok(())
}
