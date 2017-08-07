struct BinaryExpr {
    left:   Expr,
    right:  Expr
};

enum Expr {
    

}

Binary (&'static str, 

binary!(^ pow ops::Pow),
binary!(* mul ops::Mul),
binary!(+ add ops::Add),
binary!(- sub ops::Sub),

fn parse(input: &str, level: usize) {
    let mut stack = Vec::new();
    let mut iter = input.chars();
    
    for i in input.chars()(' ') {
        match i {
            "+" => parse(left?, input
    }

}
