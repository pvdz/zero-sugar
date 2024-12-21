const { transform_code } = require('./pkg/js_transformer.js');

// Example usage
const sourceCode = `
function hello() {
    console.log("Hello, world!");
}
hello();
`;

console.log('Input code:', sourceCode);

const result = transform_code(sourceCode);

if (result.had_error) {
    console.error("Error transforming code:", result.error_message);
} else {
    console.log("Transformed code:");
    console.log(result.transformed_code);
}
