import ast
import copy
import importlib
import io
import json
from pathlib import Path
import sys
import types
import unittest

root = Path.cwd()
sys.path.insert(0, str(root))
sys.path.insert(0, str(root / 'scripts'))
changes = {ast.Eq: ast.NotEq, ast.NotEq: ast.Eq, ast.Lt: ast.LtE, ast.LtE: ast.Lt, ast.Gt: ast.GtE, ast.GtE: ast.Gt, ast.Is: ast.IsNot, ast.IsNot: ast.Is, ast.In: ast.NotIn, ast.NotIn: ast.In}
report = []
for name, tests in [('medal_events', 'tests.test_medal_events'), ('publication', 'tests.test_publication')]:
    path = root / 'scripts' / (name + '.py')
    source = path.read_text('utf-8')
    tree = ast.parse(source)
    nodes = list(ast.walk(tree))
    candidates = []
    for index, node in enumerate(nodes):
        if isinstance(node, ast.Compare):
            for op_index, op in enumerate(node.ops):
                if type(op) in changes:
                    candidates.append((index, 'compare', op_index))
        elif isinstance(node, ast.BoolOp):
            candidates.append((index, 'boolean', 0))
    for index, kind, op_index in candidates:
        mutation = copy.deepcopy(tree)
        target = list(ast.walk(mutation))[index]
        if kind == 'compare':
            target.ops[op_index] = changes[type(target.ops[op_index])]()
        else:
            target.op = ast.Or() if isinstance(target.op, ast.And) else ast.And()
        module = types.ModuleType(name)
        module.__file__ = str(path)
        sys.modules[name] = module
        exec(compile(mutation, str(path), 'exec'), module.__dict__)
        if tests in sys.modules:
            test_module = importlib.reload(sys.modules[tests])
        else:
            test_module = importlib.import_module(tests)
        suite = unittest.defaultTestLoader.loadTestsFromModule(test_module)
        result = unittest.TextTestRunner(stream=io.StringIO()).run(suite)
        entry = dict(module=name, line=target.lineno, kind=kind,
                     expression=source.splitlines()[target.lineno-1].strip(), survived=result.wasSuccessful())
        report.append(entry)
    print(name + ': ' + str(sum(r['module'] == name for r in report)) + ' mutants tested', flush=True)
output = Path(sys.argv[1])
output.write_text(json.dumps(report, indent=2), encoding='utf-8')
print(json.dumps(dict(total=len(report), killed=sum(not r['survived'] for r in report), survivors=[r for r in report if r['survived']]), indent=2), flush=True)

raise SystemExit(1 if any(r['survived'] for r in report) else 0)
