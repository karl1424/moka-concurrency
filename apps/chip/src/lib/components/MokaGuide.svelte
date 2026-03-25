<script lang="ts">
  import Katex from '$lib/components/Katex.svelte';

  type Production = {
    left: string;
    right: string[][];
    inline?: boolean;
  };

  const productions: Production[] = [
    {
      left: 'Program',
      right: [['"par"', 'Command', '"[]"', '...', '"[]"', 'Command', '"rap"'], ['Command']],
    },

    {
      left: 'Command',
      right: [
        ['Var', '":="', 'AExpr'],
        ['Arr', '"["', 'AExpr', '"]"', '":="', 'AExpr'],
        ['Ch', '"!"', 'AExpr'],
        ['Ch', '"?"', 'Var'],
        ['Ch', '"?"', 'Arr', '"["', 'AExpr', '"]"'],
        ['Ch', '"!!"', 'Int', 'AExpr'],
        ['Ch', '"??"', 'Int', 'Arr', 'Var'],
        ['Ch', '"??"', 'Int', 'Arr', 'Arr', '"["', 'AExpr', '"]"'],
        ['Operation'],
        ['"skip"'],
        ['Command', '";"', 'Command'],
        ['"if"', 'CommunicationGuard', '"fi"'],
        ['"do"', 'Guard', '"ods"'],
        ['"loop"', 'CommunicationGuard', '"pool"'],
      ],
    },
    {
      left: 'Operation',
      right: [
        ['Ts', '".put"', '"("', 'Tupple', '")"'],
        ['Ts', '".get"', '"("', 'TuppleFind', '")"'],
        ['Ts', '".query"', '"("', 'TuppleFind', '")"'],
      ],
    },
    {
      left: 'Guard',
      right: [
        ['BExpr', '"->"', 'Command'],
        ['Guard', '"[]"', 'Guard'],
      ],
    },
    {
      left: 'CommunicationGuard',
      right: [
        ['Ch', '"!"', 'AExpr', '"->"', 'Command'],
        ['Ch', '"?"', 'Var', '"->"', 'Command'],
        ['Ch', '"?"', 'Arr', '"["', 'AExpr', '"]"', '"->"', 'Command'],
        ['BExpr', '"->"', 'Command'],
        ['CommunicationGuard', '"[]"', 'CommunicationGuard'],
      ],
    },
    {
      left: 'AExpr',
      right: [
        ['Int'],
        ['Var'],
        ['Arr', '"["', 'AExpr', '"]"'],
        ['"old"', '"("', 'Var', '")"'],
        ['"old"', '"("', 'Arr', '"["', 'AExpr', '"]"', '")"'],
        ['"-"', 'AExpr'],
        ['"("', 'AExpr', '")"'],
        ['AExpr', '"*"', 'AExpr'],
        ['AExpr', '"/"', 'AExpr'],
        ['AExpr', '"+"', 'AExpr'],
        ['AExpr', '"-"', 'AExpr'],
        ['Function'],
      ],
    },
    {
      left: 'BExpr',
      right: [
        ['AExpr', 'RelOp', 'AExpr'],
        ['"true"'],
        ['"false"'],
        ['"!"', 'BExpr'],
        ['"("', 'BExpr', '")"'],
        ['BExpr', '"&"', 'BExpr'],
        ['BExpr', '"|"', 'BExpr'],
        ['Ts', '".putP"', '"("', 'Tupple', '")"'],
        ['Ts', '".getP"', '"("', 'TuppleFind', '")"'],
        ['Ts', '".queryP"', '"("', 'TuppleFind', '")"'],
      ],
    },
    {
      left: 'Function',
      right: [
        ['"division"', '"("', 'AExpr', '","', 'AExpr', '")"'],
        ['"min"', '"("', 'AExpr', '","', 'AExpr', '")"'],
        ['"max"', '"("', 'AExpr', '","', 'AExpr', '")"'],
        ['"fac"', '"("', 'AExpr', '")"'],
        ['"fib"', '"("', 'AExpr', '")"'],
        ['"exp"', '"("', 'AExpr', '","', 'AExpr', '")"'],
      ],
    },
    {
      left: 'Tupple',
      right: [['AExpr'], ['AExpr', '","', 'Tupple']],
    },
    {
      left: 'Format',
      right: [['AExpr'], ['"\\_"'], ['"?"', 'Var'], ['"?"', 'Arr', '"["', 'AExpr', '"]"']],
    },
    {
      left: 'TuppleFind',
      right: [['Format'], ['Format', '","', 'TuppleFind']],
    },
    {
      left: 'RelOp',
      right: [['"<"'], ['">"'], ['"<="'], ['">="'], ['"="'], ['"!="']],
      inline: true,
    },
    {
      left: 'Var',
      right: [['r"[\\_a-zA-Z][\\_a-zA-Z0-9]*"']],
    },
    {
      left: 'Arr',
      right: [['r"[\\_a-zA-Z][\\_a-zA-Z0-9]*"']],
    },
    {
      left: 'Ch',
      right: [['r"[\\_a-zA-Z][\\_a-zA-Z0-9]*"']],
    },
    {
      left: 'Ts',
      right: [['r"[\\_a-zA-Z][\\_a-zA-Z0-9]*"']],
    },
  ];

  const pascalCaseToKebabCase = (str: string) =>
    str.replace(/([a-z0-9])([A-Z])/g, '$1-$2').toLowerCase();

  const prepareToken = (token: string) => {
    // replace "&" with "\&"
    token = token.replace(/&/g, '\\&');

    // replace "{" and "}" with "\{" and "\}"
    token = token.replace(/{/g, '\\{').replace(/}/g, '\\}');

    // replace "*$" with "^*"
    token = token.replace(/\*$/g, '^*');

    // make keywords starting and ending with " texttt
    if (token.match(/".*"/g)) {
      return `\\;\\texttt{${token}}\\;`;
    }

    // make regex bold
    if (token.match(/r"[_a-zA-Z][_a-zA-Z0-9]*"/)) {
      return `\\texttt{${token}}`;
    }

    // make non-terminals italic
    if (token.match(/[A-Z][a-zA-Z]*/)) {
      return `\\langle \\textit{${pascalCaseToKebabCase(token)}} \\rangle`;
    }

    return token;
  };

  const grammar = `
  \\begin{aligned}
      ${productions
        .map(
          (production) =>
            prepareToken(production.left) +
            ' ::= & \\;' +
            (production.inline
              ? production.right
                  .map((right) => right.map(prepareToken).join(' '))
                  .join(' \\mid  \\;')
              : production.right
                  .map((right) => right.map(prepareToken).join(' '))
                  .join(' \\\\  \\mid  & \\;')) +
            ' \\\\',
        )
        .join('')}
  \\end{aligned}
    `;
</script>

<article class="prose prose-invert mx-auto">
  <h1>Guide</h1>

  <h2>Grammar</h2>

  <Katex math={grammar} displayMode={true} />
</article>
