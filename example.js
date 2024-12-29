const { transform_code } = require('./pkg/zero_sugar.js');

// Example usage
const sourceCode = `
function hello() {
    console.log("Hello, world!");
}
hello();


"Do-while loop";
do {
    console.log(i);
    i++;
} while (i < 5);

"For loop";
for (let i = 0; i < 5; i++) {
    console.log(i);
}

`;

console.log('Input code:', sourceCode);
console.log('\n\n###############\n\n');

const result = transform_code(sourceCode);

console.log('\n\n###############\n\n');
if (result.had_error) {
    console.error("Error transforming code:", result.error_message);
} else {
    console.log("Transformed code:");
    console.log(result.transformed_code);
}
