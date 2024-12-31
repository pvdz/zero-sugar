pub struct MapperState {
    pub id_counter: usize,
}

impl MapperState {
    pub fn next_ident_name(&mut self) -> String {
        let id = self.id_counter;
        self.id_counter += 1;
        // Legit variable name AND label name :D
        format!("$zeroConfig_{}", id)
    }
}
