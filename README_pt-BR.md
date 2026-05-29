<!-- markdownlint-disable -->

<div align="center">

<img src="./src-tauri/icons/icon.png" width="120" alt="Ícone do Floral Notepaper">

# Floral Notepaper

Um aplicativo de notas adesivas leve, elegante e moderno para sua área de trabalho<br>
Construído com Tauri 2 + React

[简体中文](README.md) · [繁體中文](README_zh-HK.md)<br>
[Report an Issue](https://github.com/Achilng/floral-notepaper/issues) · [Changelog](https://github.com/Achilng/floral-notepaper/releases)

[![Version](https://img.shields.io/github/v/release/Achilng/floral-notepaper)](https://github.com/Achilng/floral-notepaper/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
![Stars](https://img.shields.io/github/stars/Achilng/floral-notepaper?color=ffcb47&labelColor=black)</br>
![React 19](https://img.shields.io/badge/React-19-blue?logo=react)
![Tauri v2](https://img.shields.io/badge/Tauri-v2-%2324C8D8?logo=tauri)
![Rust Edition 2021](https://img.shields.io/badge/Rust-2021-%23000000?logo=rust)<br>
[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/Achilng/floral-notepaper)

</div>

<!-- markdownlint-restore -->

---

## Por que Floral Notepaper

A maioria dos aplicativos de notas ou notas adesivas disponíveis é pesada e difícil de aprender, ou desatualizada e abandonada. O Floral Notepaper foi criado para ser diferente — rápido para abrir, leve de usar e agradável de ver.

## Recursos

- **Edição e Visualização em Markdown** — Compatibilidade completa com GitHub Flavored Markdown e alternância suave entre os modos de edição e visualização

  ![Captura de tela da janela principal](Docs/images/主窗口截图.png)

- **Nota Rápida** — Abra uma janela de nota instantaneamente a partir da bandeja do sistema ou através de um atalho global (padrão: `Ctrl+Space`)

  ![Exemplo de várias janelas](Docs/images/小窗多开示例.gif)

- **Modo Fixar** — Fixe uma nota em um local fixo da sua área de trabalho para referência rápida e cópia fácil

  ![Exemplo do modo fixar](Docs/images/AI绘画截图.png)

- **Importar e Exportar** — Importe e exporte notas como arquivos `.md`

## Casos de Uso

- Use como uma área de transferência sempre visível para guardar e copiar texto rapidamente
- Anote ideias enquanto joga ou assiste vídeos
- Capture um pensamento rápido ou uma inspiração repentina
- Mantenha uma lista de tarefas diretamente na sua área de trabalho

## Baixar

Acesse as [versões no GitHub](https://github.com/Achilng/floral-notepaper/releases) para baixar a versão mais recente.

## Compilando a partir do Código-Fonte

### Pré-requisitos

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://www.rust-lang.org/tools/install)
- [Tauri CLI 2](https://tauri.app/)

### Etapas

```bash
git clone https://github.com/Achilng/floral-notepaper.git
cd floral-notepaper

npm install

# Modo de desenvolvimento
npm run tauri dev

# Compilação de produção
npm run tauri build
```

Os artefatos de compilação são gerados em `src-tauri/target/release/bundle/`.

## Histórico de Estrelas

[![Star History Chart](https://api.star-history.com/svg?repos=Achilng/floral-notepaper&type=Date&legend=top-left)](https://star-history.com/#Achilng/floral-notepaper&Date)

## 🌟 Colaboradores

[![contrib.rocks](https://contrib.rocks/image?repo=Achilng/floral-notepaper&max=1000)](https://contrib.rocks/image?repo=Achilng/floral-notepaper&max=1000)

## Licença

[MIT](LICENSE)
