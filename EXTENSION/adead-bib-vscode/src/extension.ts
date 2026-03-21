import * as vscode from 'vscode';
import { exec } from 'child_process';
import * as path from 'path';

let diagnosticCollection: vscode.DiagnosticCollection;
let statusBarItem: vscode.StatusBarItem;
let outputChannel: vscode.OutputChannel;

// Palabras clave para autocompletado
const KEYWORDS = [
    'fn', 'let', 'const', 'if', 'else', 'while', 'for', 'loop', 'break', 'continue', 'return',
    'struct', 'trait', 'impl', 'enum', 'type', 'mod', 'use', 'import', 'from', 'pub',
    'self', 'Self', 'true', 'false', 'null', 'none',
    'cpu', 'gpu', 'emit', 'asm', 'hex', 'raw',
    'print', 'println', 'input', 'len', 'push', 'pop', 'get', 'set',
    'i8', 'i16', 'i32', 'i64', 'u8', 'u16', 'u32', 'u64', 'f32', 'f64', 'bool', 'str', 'char', 'void', 'int', 'float', 'string'
];

export function activate(context: vscode.ExtensionContext) {
    console.log('ADead-BIB extension activated');

    // Crear output channel
    outputChannel = vscode.window.createOutputChannel('ADead-BIB');
    context.subscriptions.push(outputChannel);

    // Crear status bar
    statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
    statusBarItem.text = '$(file-binary) ADead-BIB';
    statusBarItem.tooltip = 'ADead-BIB - Binary Is Binary';
    statusBarItem.command = 'adead-bib.showMenu';
    context.subscriptions.push(statusBarItem);

    // Crear colección de diagnósticos
    diagnosticCollection = vscode.languages.createDiagnosticCollection('adead-bib');
    context.subscriptions.push(diagnosticCollection);

    // Comando: Mostrar menú
    const menuCmd = vscode.commands.registerCommand('adead-bib.showMenu', async () => {
        const options = [
            { label: '$(play) Run', description: 'Compilar y ejecutar', command: 'adead-bib.run' },
            { label: '$(tools) Build', description: 'Compilar a ejecutable', command: 'adead-bib.build' },
            { label: '$(check) Check', description: 'Verificar sintaxis', command: 'adead-bib.check' },
            { label: '$(rocket) Optimize', description: 'Compilación ultra-optimizada', command: 'adead-bib.opt' },
            { label: '$(terminal) Playground', description: 'Modo interactivo', command: 'adead-bib.playground' }
        ];
        
        const selected = await vscode.window.showQuickPick(options, {
            placeHolder: 'Selecciona una acción ADead-BIB'
        });
        
        if (selected) {
            vscode.commands.executeCommand(selected.command);
        }
    });

    // Comando: Build
    const buildCmd = vscode.commands.registerCommand('adead-bib.build', () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor || !isAdeadBibFile(editor.document)) {
            vscode.window.showErrorMessage('No hay archivo .adB activo');
            return;
        }
        
        const file = editor.document.fileName;
        const projectRoot = getProjectRoot(file);
        runCompilerCommand('build', file, projectRoot, 'Build');
    });

    // Comando: Run
    const runCmd = vscode.commands.registerCommand('adead-bib.run', () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor || !isAdeadBibFile(editor.document)) {
            vscode.window.showErrorMessage('No hay archivo .adB activo');
            return;
        }
        
        const file = editor.document.fileName;
        const projectRoot = getProjectRoot(file);
        runCompilerCommand('run', file, projectRoot, 'Run');
    });

    // Comando: Check
    const checkCmd = vscode.commands.registerCommand('adead-bib.check', () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor || !isAdeadBibFile(editor.document)) {
            vscode.window.showErrorMessage('No hay archivo .adB activo');
            return;
        }
        
        checkSyntax(editor.document);
    });

    // Comando: Optimized Build
    const optCmd = vscode.commands.registerCommand('adead-bib.opt', () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor || !isAdeadBibFile(editor.document)) {
            vscode.window.showErrorMessage('No hay archivo .adB activo');
            return;
        }
        
        const file = editor.document.fileName;
        const projectRoot = getProjectRoot(file);
        runCompilerCommand('opt', file, projectRoot, 'Optimized Build');
    });

    // Comando: Playground
    const playCmd = vscode.commands.registerCommand('adead-bib.playground', () => {
        const projectRoot = getProjectRoot(vscode.window.activeTextEditor?.document.fileName || '');
        const terminal = vscode.window.createTerminal({
            name: 'ADead-BIB Playground',
            cwd: projectRoot
        });
        terminal.show();
        terminal.sendText(getCompilerCommand('play', '', projectRoot));
    });

    // Registrar comandos
    context.subscriptions.push(menuCmd, buildCmd, runCmd, checkCmd, optCmd, playCmd);

    // Autocompletado
    const completionProvider = vscode.languages.registerCompletionItemProvider(
        { language: 'adead-bib', scheme: 'file' },
        {
            provideCompletionItems(document: vscode.TextDocument, position: vscode.Position) {
                const completions: vscode.CompletionItem[] = [];
                
                // Palabras clave
                for (const keyword of KEYWORDS) {
                    const item = new vscode.CompletionItem(keyword, vscode.CompletionItemKind.Keyword);
                    item.detail = 'ADead-BIB keyword';
                    completions.push(item);
                }
                
                // Snippets rápidos
                const fnMain = new vscode.CompletionItem('fn main', vscode.CompletionItemKind.Snippet);
                fnMain.insertText = new vscode.SnippetString('fn main() {\n\t$0\n}');
                fnMain.detail = 'Función principal';
                completions.push(fnMain);
                
                const printlnSnip = new vscode.CompletionItem('println!', vscode.CompletionItemKind.Snippet);
                printlnSnip.insertText = new vscode.SnippetString('println("$1")');
                printlnSnip.detail = 'Imprimir con salto de línea';
                completions.push(printlnSnip);
                
                return completions;
            }
        },
        '.', ':', '('
    );
    context.subscriptions.push(completionProvider);

    // Hover provider
    const hoverProvider = vscode.languages.registerHoverProvider(
        { language: 'adead-bib', scheme: 'file' },
        {
            provideHover(document: vscode.TextDocument, position: vscode.Position) {
                const range = document.getWordRangeAtPosition(position);
                if (!range) return;
                
                const word = document.getText(range);
                
                const docs: { [key: string]: string } = {
                    'fn': '**fn** - Define una función\n\n```adB\nfn nombre(params) {\n    // código\n}\n```',
                    'let': '**let** - Declara una variable mutable\n\n```adB\nlet x = 42\n```',
                    'const': '**const** - Declara una constante\n\n```adB\nconst PI = 3\n```',
                    'println': '**println** - Imprime con salto de línea\n\n```adB\nprintln("Hola mundo")\n```',
                    'print': '**print** - Imprime sin salto de línea\n\n```adB\nprint("Texto")\n```',
                    'if': '**if** - Condicional\n\n```adB\nif condicion {\n    // código\n}\n```',
                    'while': '**while** - Bucle while\n\n```adB\nwhile condicion {\n    // código\n}\n```',
                    'for': '**for** - Bucle for\n\n```adB\nfor i in 0..10 {\n    // código\n}\n```',
                    'struct': '**struct** - Define una estructura\n\n```adB\nstruct Punto {\n    x: i32,\n    y: i32\n}\n```',
                    'impl': '**impl** - Implementación de métodos\n\n```adB\nimpl Punto {\n    fn new() { }\n}\n```',
                    'cpu': '**cpu::** - Bloque de código CPU directo\n\n```adB\ncpu:: {\n    // instrucciones x86-64\n}\n```',
                    'gpu': '**gpu::** - Bloque de código GPU directo\n\n```adB\ngpu:: {\n    // shaders/compute\n}\n```',
                    'emit': '**emit![]** - Emite bytes HEX directos\n\n```adB\nemit![0x90, 0xC3]\n```',
                    'return': '**return** - Retorna un valor de la función',
                    'true': '**true** - Valor booleano verdadero',
                    'false': '**false** - Valor booleano falso'
                };
                
                if (docs[word]) {
                    return new vscode.Hover(new vscode.MarkdownString(docs[word]));
                }
                
                return null;
            }
        }
    );
    context.subscriptions.push(hoverProvider);

    // Diagnósticos en tiempo real
    vscode.workspace.onDidSaveTextDocument((document) => {
        if (isAdeadBibFile(document)) {
            checkSyntax(document);
        }
    });

    vscode.workspace.onDidOpenTextDocument((document) => {
        if (isAdeadBibFile(document)) {
            statusBarItem.show();
        }
    });

    vscode.window.onDidChangeActiveTextEditor((editor) => {
        if (editor && isAdeadBibFile(editor.document)) {
            statusBarItem.show();
        } else {
            statusBarItem.hide();
        }
    });

    // Mostrar status bar si hay archivo .adB abierto
    if (vscode.window.activeTextEditor && isAdeadBibFile(vscode.window.activeTextEditor.document)) {
        statusBarItem.show();
        checkSyntax(vscode.window.activeTextEditor.document);
    }
}

function isAdeadBibFile(document: vscode.TextDocument): boolean {
    return document.languageId === 'adead-bib' || document.fileName.endsWith('.adB');
}

function getProjectRoot(filePath: string): string {
    // Buscar el directorio raíz del proyecto ADead-BIB
    let dir = path.dirname(filePath);
    
    // Buscar hacia arriba hasta encontrar Cargo.toml o src/rust
    for (let i = 0; i < 10; i++) {
        const cargoPath = path.join(dir, 'Cargo.toml');
        const srcPath = path.join(dir, 'src', 'rust');
        
        try {
            // Si existe Cargo.toml, es el root
            if (require('fs').existsSync(cargoPath)) {
                return dir;
            }
        } catch {}
        
        const parent = path.dirname(dir);
        if (parent === dir) break;
        dir = parent;
    }
    
    // Fallback: usar el directorio del archivo
    return path.dirname(filePath);
}

function getCompilerCommand(action: string, file: string, projectRoot: string): string {
    const config = vscode.workspace.getConfiguration('adead-bib');
    const compilerPath = config.get<string>('compilerPath') || '';
    
    // Si hay un path configurado, usarlo
    if (compilerPath && compilerPath !== 'adB') {
        return `"${compilerPath}" ${action} "${file}"`;
    }
    
    // Intentar usar cargo run desde el proyecto
    const cargoPath = path.join(projectRoot, 'Cargo.toml');
    try {
        if (require('fs').existsSync(cargoPath)) {
            if (file) {
                return `cargo run --bin adeadc -- ${action} "${file}"`;
            } else {
                return `cargo run --bin adeadc -- ${action}`;
            }
        }
    } catch {}
    
    // Fallback: intentar adeadc directamente
    if (file) {
        return `adeadc ${action} "${file}"`;
    }
    return `adeadc ${action}`;
}

function runCompilerCommand(action: string, file: string, projectRoot: string, taskName: string) {
    const command = getCompilerCommand(action, file, projectRoot);
    
    outputChannel.appendLine(`[${new Date().toLocaleTimeString()}] ${taskName}: ${command}`);
    outputChannel.show(true);
    
    const terminal = vscode.window.createTerminal({
        name: `ADead-BIB: ${taskName}`,
        cwd: projectRoot
    });
    terminal.show();
    terminal.sendText(command);
}

function checkSyntax(document: vscode.TextDocument) {
    const file = document.fileName;
    const projectRoot = getProjectRoot(file);
    const command = getCompilerCommand('check', file, projectRoot) + ' --json';
    
    exec(command, { cwd: projectRoot }, (err, stdout, stderr) => {
        diagnosticCollection.delete(document.uri);
        
        if (err && !stdout) {
            // No mostrar error si el compilador no está disponible
            statusBarItem.text = '$(warning) ADead-BIB';
            statusBarItem.tooltip = 'Compilador no encontrado. Configura adead-bib.compilerPath';
            return;
        }
        
        try {
            const result = JSON.parse(stdout);
            const diagnostics: vscode.Diagnostic[] = [];
            
            // Procesar errores
            if (result.errors) {
                for (const error of result.errors) {
                    const range = new vscode.Range(
                        (error.line || 1) - 1, (error.column || 1) - 1,
                        (error.line || 1) - 1, 1000
                    );
                    
                    const diagnostic = new vscode.Diagnostic(
                        range,
                        error.message || 'Error de sintaxis',
                        vscode.DiagnosticSeverity.Error
                    );
                    diagnostic.source = 'ADead-BIB';
                    diagnostics.push(diagnostic);
                }
            }
            
            // Procesar warnings
            if (result.warnings) {
                for (const warning of result.warnings) {
                    const range = new vscode.Range(
                        (warning.line || 1) - 1, (warning.column || 1) - 1,
                        (warning.line || 1) - 1, 1000
                    );
                    
                    let severity = vscode.DiagnosticSeverity.Warning;
                    if (warning.severity === 'info') {
                        severity = vscode.DiagnosticSeverity.Information;
                    } else if (warning.severity === 'hint') {
                        severity = vscode.DiagnosticSeverity.Hint;
                    }
                    
                    const diagnostic = new vscode.Diagnostic(
                        range,
                        warning.message || 'Warning',
                        severity
                    );
                    diagnostic.source = 'ADead-BIB';
                    diagnostic.code = warning.type;
                    diagnostics.push(diagnostic);
                }
            }
            
            diagnosticCollection.set(document.uri, diagnostics);
            
            // Actualizar status bar
            if (result.status === 'ok') {
                const diag = result.diagnostics;
                if (diag) {
                    statusBarItem.text = `$(check) ADead-BIB: ${diag.functions} fn`;
                    statusBarItem.tooltip = `${diag.functions} funciones, ${diag.variables} variables`;
                }
            } else {
                statusBarItem.text = '$(error) ADead-BIB';
            }
            
        } catch (parseError) {
            console.error('Error parsing JSON:', parseError, stdout);
        }
    });
}

export function deactivate() {
    if (diagnosticCollection) {
        diagnosticCollection.dispose();
    }
    if (statusBarItem) {
        statusBarItem.dispose();
    }
    if (outputChannel) {
        outputChannel.dispose();
    }
}
