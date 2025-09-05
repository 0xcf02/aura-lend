module.exports = {
  env: {
    browser: true,
    es2021: true,
    node: true,
    mocha: true,
  },
  extends: [
    'eslint:recommended',
    '@typescript-eslint/recommended',
  ],
  parser: '@typescript-eslint/parser',
  parserOptions: {
    ecmaVersion: 'latest',
    sourceType: 'module',
    project: './tsconfig.json',
  },
  plugins: [
    '@typescript-eslint',
  ],
  rules: {
    // TypeScript specific rules
    '@typescript-eslint/no-explicit-any': 'warn',
    '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_' }],
    '@typescript-eslint/explicit-function-return-type': 'off',
    '@typescript-eslint/explicit-module-boundary-types': 'off',
    '@typescript-eslint/no-non-null-assertion': 'warn',
    
    // Import organization
    'sort-imports': ['error', { 
      ignoreCase: true, 
      ignoreDeclarationSort: true,
      allowSeparatedGroups: true 
    }],
    
    // Code quality
    'no-console': 'warn',
    'no-debugger': 'error',
    'no-duplicate-imports': 'error',
    'prefer-const': 'error',
    'no-var': 'error',
    
    // Solana/Anchor specific patterns
    'no-bitwise': 'off', // Bitwise operations are common in blockchain
    'no-magic-numbers': 'off', // Blockchain often uses specific numbers
    
    // Formatting (handled by Prettier)
    'indent': 'off',
    'quotes': 'off',
    'semi': 'off',
  },
  overrides: [
    {
      files: ['tests/**/*.ts'],
      rules: {
        '@typescript-eslint/no-explicit-any': 'off',
        'no-console': 'off',
      },
    },
    {
      files: ['sdk/src/idl/*.ts'],
      rules: {
        '@typescript-eslint/no-explicit-any': 'off',
        'sort-imports': 'off',
      },
    },
  ],
  ignorePatterns: [
    'node_modules/',
    'lib/',
    'target/',
    '*.js',
    'programs/**/*.rs',
  ],
};