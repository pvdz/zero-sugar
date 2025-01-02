pub struct MapperState {
    pub id_counter: usize,

    // We need to track while loops and any parent labels
    // For each we need to track whether they are targeted
    // by a continue statement.
    //
    // If they are, we need to generate a new label for the break
    // statement that will replace the continue statement.
    //
    // We also need to track the label of the continue statement
    // so we can generate the correct break statement.
    //
    // Since labels and while nesting is hierarchical, we can
    // use a stack to track the current while loop and any parent
    // labels and keep their "used" state in sync. Since we need
    // to know where a certain label points to, we use the same
    // trick as the finally transform and use "#loop" as a token
    // to indicate a while loop statement versus its parent label.
    //
    // When we enter a label that is a direct parent of a while
    // loop, or enter a while loop, we push a new entry onto the
    // stack. We pop it when we leave either. This should keep
    // the stack in sync with the while loop and label nesting.
    //
    // When leaving a while loop, we need to check if the continue
    // statement is targeting it or a parent label. It is when the
    // second element of the tuple is not empty. In that case it
    // will contain a generated label. This way multiple continues
    // targeting the same loop can use the same generated label.
    // Labels should always check the generated label of the next
    // entry in the stack, which must be a while loop as per syntax.
    //
    // So this vector means: Vec<(while_label_name, Option<generated_label_name>)>
    pub continue_targets: Vec<(String, Option<String>)>,
}

impl MapperState {
    pub fn next_ident_name(&mut self) -> String {
        let id = self.id_counter;
        self.id_counter += 1;
        // Legit variable name AND label name :D
        format!("$zeroSugar{}", id)
    }
}
