# 3D New Era AI

[English](README.md) · **Português (Brasil)** · **[Site](https://3dneweraai.com/)** · **[Usar no navegador](https://3dneweraai.com/app/)**

[![CI](https://github.com/leandrodaf/3d-new-era-ai/actions/workflows/ci.yml/badge.svg)](https://github.com/leandrodaf/3d-new-era-ai/actions/workflows/ci.yml)
[![Última versão](https://img.shields.io/github/v/release/leandrodaf/3d-new-era-ai?label=vers%C3%A3o)](https://github.com/leandrodaf/3d-new-era-ai/releases/latest)
[![Licença: MIT OU Apache-2.0](https://img.shields.io/badge/licen%C3%A7a-MIT%20OU%20Apache--2.0-blue.svg)](#licença)

Um editor de **projetos de casa, planta baixa e interiores** de código aberto, feito em
Rust para que **a sua IA projete junto com você**. Ele vem com um servidor
[Model Context Protocol](https://modelcontextprotocol.io): o Claude, o ChatGPT, o Codex, o
Gemini, o Cursor ou qualquer outro cliente MCP desenha as paredes, mobilia os cômodos,
projeta a marcenaria, confere iluminação e ergonomia pelas normas e renderiza as fotos,
com cada alteração aparecendo na tela na hora e a um Ctrl+Z de distância.

Uma alternativa ao Sweet Home 3D para Windows, macOS, Linux e navegador, em português,
inglês, espanhol e francês.

![Editor com um apartamento de 105 m² mobiliado: catálogo, planta humanizada e vista 3D ao vivo](docs/images/editor.png)

## Três jeitos de usar

| | O que você tem | Conta |
|---|---|---|
| **[App para computador](#instalar)** | O editor completo na sua máquina, com um servidor MCP local em `127.0.0.1:7878`. Nada sai do seu computador. | não precisa |
| **[Editor no navegador](https://3dneweraai.com/app/)** | O mesmo editor numa aba (WebGPU ou WebGL). Ligue o MCP dele e a sua IA edita o projeto da aba. | não precisa |
| **[Conector na nuvem](https://3dneweraai.com/connector/)** | Adicione `https://mcp.3dneweraai.com/mcp` no Claude ou no ChatGPT e projete pela conversa, com os projetos guardados na sua conta. | entrar com e-mail |

O app e o editor no navegador são gratuitos, de código aberto e não pedem conta. O
conector na nuvem é gratuito dentro de uma cota; veja os [preços](https://3dneweraai.com/pricing/).

## Instalar

**Windows** — no PowerShell:

```powershell
irm https://raw.githubusercontent.com/leandrodaf/3d-new-era-ai/main/scripts/install-windows.ps1 | iex
```

**macOS** (Apple Silicon e Intel) — no Terminal:

```sh
curl -fsSL https://raw.githubusercontent.com/leandrodaf/3d-new-era-ai/main/scripts/install-macos.sh | bash
```

ou com o Homebrew: `brew install --cask leandrodaf/tap/3d-new-era-ai`.

Os instaladores valem só para o seu usuário, sem senha de administrador: o app aparece no
Menu Iniciar ou no Launchpad, o `newera` entra no `PATH`, o servidor MCP é registrado no
Claude Code e no Codex se você os tiver, e o mesmo comando atualiza. Para desinstalar, use
`install-macos.sh --uninstall` ou *Configurações → Aplicativos* no Windows.

Prefere baixar o arquivo?

| [Windows (x64)](https://github.com/leandrodaf/3d-new-era-ai/releases/latest/download/newera-windows-x64.zip) | [macOS Apple Silicon](https://github.com/leandrodaf/3d-new-era-ai/releases/latest/download/newera-macos-apple-silicon.zip) | [macOS Intel](https://github.com/leandrodaf/3d-new-era-ai/releases/latest/download/newera-macos-intel.zip) | [Linux (x64)](https://github.com/leandrodaf/3d-new-era-ai/releases/latest/download/newera-linux-x64.tar.gz) |
|:---:|:---:|:---:|:---:|

<details>
<summary>Primeira abertura de um arquivo baixado</summary>

Os programas não são assinados com certificado pago da Apple ou da Microsoft, então o
sistema pergunta uma vez:

- **Windows:** descompacte e abra o `newera-gui.exe`; se aparecer o SmartScreen, clique em
  *Mais informações* → *Executar assim mesmo*.
- **macOS:** descompacte e mova o *3D New Era AI* para Aplicativos, depois clique com o
  botão direito → *Abrir* → *Abrir*. No macOS 15 ou mais novo, tente abrir uma vez e vá em
  *Ajustes do Sistema* → *Privacidade e Segurança* → *Abrir Mesmo Assim*, ou rode
  `xattr -dr com.apple.quarantine "/Applications/3D New Era AI.app"`.
- **Linux:** `tar xzf newera-linux-x64.tar.gz && ./newera/newera`. Rode
  `./newera/install-desktop.sh` para pôr o app no menu, com ícone, e dar aos arquivos
  `.newera` ícone e duplo clique (`--uninstall` desfaz). Pede glibc 2.35 ou mais nova:
  Ubuntu 22.04, Debian 12, Fedora 36 ou qualquer versão posterior.

Os instaladores de uma linha evitam esses avisos.
</details>

## Conecte sua IA

**Na nuvem, sem instalar nada.** No Claude ou no ChatGPT, adicione um conector
personalizado com o endereço `https://mcp.3dneweraai.com/mcp` e entre com o seu e-mail.
Passo a passo: <https://3dneweraai.com/connector/>.

**Com o app.** Enquanto o editor está aberto, ele serve o MCP em
`http://127.0.0.1:7878/mcp` (ou rode `newera serve` para ficar sem janela). O menu **IA**,
ou o indicador de MCP na barra de status, mostra a configuração do seu cliente, pode
registrá-lo para você e lista as ferramentas que o agente chama, na hora em que chama.

**Numa aba do navegador.** Abra <https://3dneweraai.com/app/> e ligue o MCP no painel
**IA** (Ctrl+Shift+M). A aba ganha um endereço que a sua IA alcança por um pequeno relay,
porque uma aba não consegue abrir uma porta. O relay só repassa as mensagens e não guarda
nada; o projeto fica na aba, e o endereço para de responder quando você desliga ou fecha a
aba. Trate esse endereço como uma senha. Vídeo não é oferecido ali, porque travaria a aba
por minutos.

Depois é só pedir, por exemplo: *"Desenhe um quarto de 4 × 5 m com porta e janela,
mobilie e gere uma foto."*

### Configurar o cliente

Um clique, com o app instalado:

[![Install in Cursor](https://cursor.com/deeplink/mcp-install-dark.svg)](https://cursor.com/en/install-mcp?name=newera&config=eyJjb21tYW5kIjoibmV3ZXJhIiwiYXJncyI6WyJtY3AiXX0%3D)
[![Install in VS Code](https://img.shields.io/badge/VS_Code-Install_newera-0098FF?logo=visualstudiocode&logoColor=white)](https://insiders.vscode.dev/redirect/mcp/install?name=newera&config=%7B%22type%22%3A%22stdio%22%2C%22command%22%3A%22newera%22%2C%22args%22%3A%5B%22mcp%22%5D%7D)
[![Install in VS Code Insiders](https://img.shields.io/badge/VS_Code_Insiders-Install_newera-24bfa5?logo=visualstudiocode&logoColor=white)](https://insiders.vscode.dev/redirect/mcp/install?name=newera&config=%7B%22type%22%3A%22stdio%22%2C%22command%22%3A%22newera%22%2C%22args%22%3A%5B%22mcp%22%5D%7D&quality=insiders)

**Claude Desktop:** baixe o `newera-mcp.mcpb` da
[última versão](https://github.com/leandrodaf/3d-new-era-ai/releases/latest) e abra.

**Plugins com skills:**

| Cliente | Comando |
|---|---|
| Claude Code | `/plugin marketplace add leandrodaf/3d-new-era-ai`, depois `/plugin install 3d-new-era-ai-desktop@3d-new-era-ai` (o app na sua tela) ou `/plugin install 3d-new-era-ai@3d-new-era-ai` (na nuvem) |
| Codex | `codex plugin marketplace add leandrodaf/3d-new-era-ai` |
| Gemini CLI | `gemini extensions install https://github.com/leandrodaf/3d-new-era-ai` |

<details>
<summary>Configuração manual de cada cliente</summary>

**Claude Code** (dentro de um clone deste repositório, o `.mcp.json` já configura)

```sh
claude mcp add --transport http newera http://127.0.0.1:7878/mcp
```

**Codex CLI**

```sh
codex mcp add newera --url http://127.0.0.1:7878/mcp
```

**Gemini CLI**

```sh
gemini mcp add --transport http newera http://127.0.0.1:7878/mcp
```

**VS Code (modo agente do Copilot)**

```sh
code --add-mcp '{"name":"newera","type":"http","url":"http://127.0.0.1:7878/mcp"}'
```

**Cursor** — `~/.cursor/mcp.json`

```json
{ "mcpServers": { "newera": { "url": "http://127.0.0.1:7878/mcp" } } }
```

**Windsurf** — `~/.codeium/windsurf/mcp_config.json`

```json
{ "mcpServers": { "newera": { "serverUrl": "http://127.0.0.1:7878/mcp" } } }
```

**Claude Desktop sem o pacote** — *Configurações → Desenvolvedor → Editar configuração*
(precisa do [Node.js](https://nodejs.org) para a ponte `mcp-remote`)

```json
{ "mcpServers": { "newera": { "command": "npx", "args": ["-y", "mcp-remote", "http://127.0.0.1:7878/mcp"] } } }
```

**DeepSeek, Qwen, Llama e outros modelos** — o que importa não é o modelo, e sim o app em
que você conversa. Use um que suporte MCP: [Cline](https://cline.bot) ou
[Roo Code](https://roocode.com) no VS Code, [Cherry Studio](https://cherry-ai.com),
[LM Studio](https://lmstudio.ai) ou [opencode](https://opencode.ai) (`opencode.json`):

```json
{ "mcp": { "newera": { "type": "remote", "url": "http://127.0.0.1:7878/mcp" } } }
```

**Qualquer outro cliente** — HTTP (Streamable) em `http://127.0.0.1:7878/mcp` ou, para
clientes que só iniciam um processo, stdio:

```json
{ "mcpServers": { "newera": { "command": "newera", "args": ["mcp"] } } }
```

Pelo stdio, com o editor aberto o agente edita a planta dessa janela; sem janela (ou com
`newera mcp --standalone`) ele trabalha num projeto próprio.
</details>

## Vitrine

Um apartamento real de 105 m², redesenhado a partir de uma planta com licença aberta,
mobiliado, iluminado e fotografado por um agente de IA só pelo MCP: paredes tiradas da
imagem, portas abrindo para o lado certo, painel ripado da TV, cozinha em L e armários
feitos com `cabinet_run`, spots dimensionados por fotometria pela NBR ISO/CIE 8995-1 e
fotos com path tracing. Todas as imagens saíram do próprio editor.
**[Veja como foi feito →](docs/SHOWCASE.md)** (em inglês)

| Planta humanizada com os acabamentos reais | Vista aérea em corte |
|---|---|
| ![Planta humanizada com acabamentos reais](docs/images/showcase/13-rendered-plan.jpg) | ![Vista aérea em corte do apartamento](docs/images/showcase/12-aerial.jpg) |

| De dia | Ao anoitecer, luzes acesas |
|---|---|
| ![Sala com painel ripado da TV](docs/images/showcase/01-living-window.jpg) | ![Mesa de jantar sob o pendente ao anoitecer](docs/images/showcase/09-dining-night.jpg) |
| ![Jantar e estar olhando para a janela](docs/images/showcase/02-dining-living.jpg) | ![Cozinha em L à noite](docs/images/showcase/10-kitchen-night.jpg) |
| ![Cozinha em L com piso de granilite](docs/images/showcase/04-kitchen.jpg) | ![Suíte do casal à noite](docs/images/showcase/11-master-bedroom-night.jpg) |

## O que ele faz

- **Planta a partir de qualquer coisa.** Desenhe paredes retas, curvas e inclinadas, ou
  coloque uma planta escaneada em escala real e deixe o `trace_background` achar as
  paredes. Abre projetos do Sweet Home 3D (`.sh3d`) como estão.
- **Marcenaria que a oficina consegue fazer.** Armários, guarda-roupas, painéis ripados,
  bancadas com recortes exatos de cuba e cooktop, sancas com LED e sofás modulares, com
  lista de corte em CSV ou DXF/SVG.
- **Conferido para quem vai morar.** Revisão de ergonomia (circulação, camas e banheiros
  por pessoa, triângulo da cozinha, giro de cadeira de rodas) e lux por cômodo por
  fotometria, cada achado ligado à [norma de origem](docs/STANDARDS.md) e com a correção
  pronta para aplicar.
- **Projetos elétrico e hidráulico** sobre a mesma planta, pela NBR 5410, NBR 5626 e
  NBR 8160.
- **Imagens que vendem o projeto.** Planta humanizada, elevações e cortes, vista aérea em
  corte, fotos com o sol da bússola e as luzes que você colocou, e vídeos por um caminho de
  câmera. Tudo renderiza na CPU, até num servidor sem tela.
- **Alternativas sem medo.** Duplique a planta numa aba nova, mude e compare lado a lado.

A lista completa de ferramentas MCP, os modos de linha de comando e como compilar estão no
[README em inglês](README.md).

## Privacidade e telemetria

O app envia relatórios de erro pelo Sentry, junto com as notas que os agentes deixam pela
ferramenta `feedback` do MCP. Vem **ligada por padrão** e desliga com um clique em
**Ajuda → Enviar relatórios de erro** ou com `newera telemetry off`. Nada do projeto é
enviado, nem o IP nem o nome da máquina. Pela mesma chave, abrir o app conta um `app_open` —
id sorteado da instalação, versão, sistema, arquitetura e modo —, o pedido de apoio conta um
`support_prompt` quando aparece e qual botão o fechou, e os instaladores contam uma
instalação e uma desinstalação, dizendo o sistema e se o app veio de uma versão publicada ou
foi compilado ali. As duas contagens vão para `3dneweraai.com/ping`, um endereço do site que
guarda o segredo do Google Analytics e repassa a contagem sem levar o IP adiante: vai só o
país. Um build feito sem as chaves do projeto conta pelo mesmo endereço, então instalar
compilando conta como qualquer outra instalação; `cargo build` num clone e a integração
contínua não contam nada. O conector na nuvem guarda só o necessário: e-mail, projetos e uso,
nunca a conversa. Veja a [política de privacidade](https://3dneweraai.com/privacy/).

Fora dessa chave, o app desktop pergunta à API pública do GitHub qual é a última versão ao abrir,
sem enviar nada sobre a instalação, e avisa na barra de status quando há uma mais nova.

## Apoie o projeto

Tudo acima é gratuito e continua gratuito. Se o projeto te poupa tempo, o
[plano Supporter](https://3dneweraai.com/pricing/) (US$ 5 por mês ou US$ 48 por ano)
amplia as cotas da nuvem na sua conta e paga o servidor.

## Contribuir

Contribuições são bem-vindas. O guia está em [CONTRIBUTING.md](CONTRIBUTING.md) (em
inglês); `make check` roda as mesmas verificações do CI.

## Licença

Licenciado sob a [Apache License 2.0](LICENSE-APACHE) ou a [licença MIT](LICENSE-MIT), à
sua escolha.
