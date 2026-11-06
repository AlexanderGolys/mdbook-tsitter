# Semantic tokens: rust-analyzer

A grammar sees syntax. Every capitalised name in the code below is a *type* to
tree-sitter, whether it is a struct, a trait, a generic parameter or an
associated type, and every call is a *call*. rust-analyzer resolved the code, so
it knows which is which, and which names are mutable, which are consumed, and
which belong to the standard library.

The three panes hold the same code. The first is mdBook's highlight.js, the
second tree-sitter alone, the third tree-sitter with the tokens rust-analyzer
sent for the file. Look at `Service` and `Send` (traits), `Request` and `Pin`
(structs), `Option` (an enum), `Inner` (a type parameter), `Self::Future` (an
associated type), `timing` (mutable) and `match`, `if` and `.await` (control
flow).

````rust,noplayground,pane="highlight.js",notreesitter
/// Wraps the language service, ending the session when the client sends the
/// `exit` notification and recording whether `shutdown` preceded it.
///
/// ```
/// use session::{ExitNotificationService, SessionEnd};
///
/// let inner = ();
/// let service = ExitNotificationService::new(inner, SessionEnd::default());
/// ```
#[derive(Debug, Clone)]
pub struct ExitNotificationService<Inner> {
    /// Wrapped service receiving requests after interception.
    inner: Inner,
    /// Shared shutdown state for the client session, see [`SessionEnd`].
    session_end: SessionEnd,
}

impl<Inner> ExitNotificationService<Inner> {
    /// Wraps the supplied transport component with the shared session-shutdown state.
    pub fn new(inner: Inner, session_end: SessionEnd) -> Self {
        Self { inner, session_end }
    }
}

impl<Inner> Service<Request> for ExitNotificationService<Inner>
where
    Inner: Service<Request, Response = Option<Response>>,
    Inner::Future: Send + 'static,
{
    type Response = Option<Response>;
    type Error = Inner::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    #[tracing::instrument(level = "debug", skip_all)]
    fn call(&mut self, request: Request) -> Self::Future {
        match request.method() {
            "shutdown" => self.session_end.record_shutdown_request(),
            "exit" => self.session_end.end(),
            _ => {}
        }
        let mut timing = crate::telemetry::Timing::start(
            "rpc",
            serde_json::json!({
                "method": request.method(), "request_id": request.id(),
                "uri": request.params().and_then(|params| params.pointer("/textDocument/uri")),
                "version": request.params().and_then(|params| params.pointer("/textDocument/version")),
                "status": "cancelled",
            }),
        );
        let future = self.inner.call(request);
        Box::pin(async move {
            let response = future.await;
            timing.record("status", "completed");
            match &response {
                Ok(Some(response)) => {
                    if let Some(error) = response.error() {
                        timing.error(
                            serde_json::json!({"code": error.code, "message": error.message}),
                        );
                    }
                }
                Err(_) => timing.error(serde_json::json!({"kind": "transport"})),
                _ => {}
            }
            response
        })
    }
}
````

````rust,noplayground,pane="tree-sitter"
/// Wraps the language service, ending the session when the client sends the
/// `exit` notification and recording whether `shutdown` preceded it.
///
/// ```
/// use session::{ExitNotificationService, SessionEnd};
///
/// let inner = ();
/// let service = ExitNotificationService::new(inner, SessionEnd::default());
/// ```
#[derive(Debug, Clone)]
pub struct ExitNotificationService<Inner> {
    /// Wrapped service receiving requests after interception.
    inner: Inner,
    /// Shared shutdown state for the client session, see [`SessionEnd`].
    session_end: SessionEnd,
}

impl<Inner> ExitNotificationService<Inner> {
    /// Wraps the supplied transport component with the shared session-shutdown state.
    pub fn new(inner: Inner, session_end: SessionEnd) -> Self {
        Self { inner, session_end }
    }
}

impl<Inner> Service<Request> for ExitNotificationService<Inner>
where
    Inner: Service<Request, Response = Option<Response>>,
    Inner::Future: Send + 'static,
{
    type Response = Option<Response>;
    type Error = Inner::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    #[tracing::instrument(level = "debug", skip_all)]
    fn call(&mut self, request: Request) -> Self::Future {
        match request.method() {
            "shutdown" => self.session_end.record_shutdown_request(),
            "exit" => self.session_end.end(),
            _ => {}
        }
        let mut timing = crate::telemetry::Timing::start(
            "rpc",
            serde_json::json!({
                "method": request.method(), "request_id": request.id(),
                "uri": request.params().and_then(|params| params.pointer("/textDocument/uri")),
                "version": request.params().and_then(|params| params.pointer("/textDocument/version")),
                "status": "cancelled",
            }),
        );
        let future = self.inner.call(request);
        Box::pin(async move {
            let response = future.await;
            timing.record("status", "completed");
            match &response {
                Ok(Some(response)) => {
                    if let Some(error) = response.error() {
                        timing.error(
                            serde_json::json!({"code": error.code, "message": error.message}),
                        );
                    }
                }
                Err(_) => timing.error(serde_json::json!({"kind": "transport"})),
                _ => {}
            }
            response
        })
    }
}
````

````rust,noplayground,lsp,pane="tree-sitter + rust-analyzer"
/// Wraps the language service, ending the session when the client sends the
/// `exit` notification and recording whether `shutdown` preceded it.
///
/// ```
/// §keyword.injected|use§ §namespace.injected|session§::{§struct.injected|ExitNotificationService§, §struct.injected|SessionEnd§};
///
/// §keyword.injected|let§ §variable.declaration.injected|inner§ = ();
/// §keyword.injected|let§ §variable.declaration.injected|service§ = §struct.injected|ExitNotificationService§::§function.injected|new§(§variable.consuming.injected|inner§, §struct.injected|SessionEnd§::§function.injected|default§());
/// ```
#[§decorator.defaultLibrary|derive§(§derive.defaultLibrary|Debug§, §derive.defaultLibrary|Clone§)]
pub struct §struct.declaration|ExitNotificationService§<§typeParameter.declaration|Inner§> {
    /// Wrapped service receiving requests after interception.
    §property.declaration|inner§: §typeParameter|Inner§,
    /// Shared shutdown state for the client session, see §struct.documentation.injected.intraDocLink|[`SessionEnd`]§.
    §property.declaration|session_end§: §struct|SessionEnd§,
}

impl<§typeParameter.declaration|Inner§> §struct|ExitNotificationService§<§typeParameter|Inner§> {
    /// Wraps the supplied transport component with the shared session-shutdown state.
    pub fn §function.declaration.static|new§(§parameter.declaration|inner§: §typeParameter|Inner§, §parameter.declaration|session_end§: §struct|SessionEnd§) -> §selfTypeKeyword|Self§ {
        §selfTypeKeyword|Self§ { §property|inner§, §property|session_end§ }
    }
}

impl<§typeParameter.declaration|Inner§> §interface|Service§<§struct|Request§> for §struct|ExitNotificationService§<§typeParameter|Inner§>
where
    §typeParameter|Inner§: §interface|Service§<§struct|Request§, §typeAlias.static|Response§ = §enum.defaultLibrary|Option§<§struct|Response§>>,
    §typeParameter|Inner§::§typeAlias.static|Future§: §interface.defaultLibrary|Send§ + §lifetime|'static§,
{
    type §typeAlias.declaration.static|Response§ = §enum.defaultLibrary|Option§<§struct|Response§>;
    type §typeAlias.declaration.static|Error§ = §typeParameter|Inner§::§typeAlias.static|Error§;
    type §typeAlias.declaration.static|Future§ = §struct.defaultLibrary|Pin§<§struct.defaultLibrary|Box§<dyn §interface.defaultLibrary|Future§<§typeAlias.static.defaultLibrary|Output§ = §enum.defaultLibrary|Result§<§selfTypeKeyword|Self§::§typeAlias.static|Response§, §selfTypeKeyword|Self§::§typeAlias.static|Error§>> + §interface.defaultLibrary|Send§>>;

    fn §method.declaration.mutable|poll_ready§(&mut §selfKeyword.declaration.mutable|self§, §parameter.declaration.mutable|cx§: &mut §struct.defaultLibrary|Context§<§lifetime|'_§>) -> §enum.defaultLibrary|Poll§<§enum.defaultLibrary|Result§<(), §selfTypeKeyword|Self§::§typeAlias.static|Error§>> {
        §selfKeyword.mutable|self§.§property|inner§.§method.mutable|poll_ready§(§parameter.consuming.mutable|cx§)
    }

    #[§namespace.crateRoot|tracing§::§decorator|instrument§(§generic|level§ = "debug", §generic|skip_all§)]
    fn §method.declaration.mutable|call§(&mut §selfKeyword.declaration.mutable|self§, §parameter.declaration|request§: §struct|Request§) -> §selfTypeKeyword|Self§::§typeAlias.static|Future§ {
        §keyword.controlFlow|match§ §parameter|request§.§method|method§() {
            "shutdown" => §selfKeyword.mutable|self§.§property|session_end§.§method|record_shutdown_request§(),
            "exit" => §selfKeyword.mutable|self§.§property|session_end§.§method|end§(),
            _ => {}
        }
        let mut §variable.declaration.mutable|timing§ = §keyword.crateRoot|crate§::§namespace|telemetry§::§struct|Timing§::§function.static|start§(
            "rpc",
            §namespace.crateRoot|serde_json§::§macro|json§!({
                "method": §parameter|request§.§method|method§(), "request_id": §parameter|request§.§method|id§(),
                "uri": §parameter|request§.§method|params§().§method.defaultLibrary.constant|and_then§(|§parameter.declaration|params§| §parameter|params§.§method|pointer§("/textDocument/uri")),
                "version": §parameter|request§.§method|params§().§method.defaultLibrary.constant|and_then§(|§parameter.declaration|params§| §parameter|params§.§method|pointer§("/textDocument/version")),
                "status": "cancelled",
            }),
        );
        let §variable.declaration|future§ = §selfKeyword.mutable|self§.§property|inner§.§method.mutable|call§(§parameter.consuming|request§);
        §struct.defaultLibrary|Box§::§unresolvedReference|pin§(§keyword.async|async§ move {
            let §variable.declaration|response§ = §variable|future§.§keyword.async.controlFlow|await§;
            §variable.mutable|timing§.§method.mutable|record§("status", "completed");
            §keyword.controlFlow|match§ &§variable|response§ {
                §enumMember.defaultLibrary|Ok§(§enumMember.defaultLibrary|Some§(§variable.declaration|response§)) => {
                    §keyword.controlFlow|if§ let §enumMember.defaultLibrary|Some§(§variable.declaration|error§) = §variable|response§.§method.constant|error§() {
                        §variable.mutable|timing§.§method.mutable|error§(
                            §namespace.crateRoot|serde_json§::§macro|json§!({"code"§keyword.async|:§ §variable|error§.§property|code§, "message"§keyword.async|:§ §variable|error§.§property|message§}),
                        );
                    }
                }
                §enumMember.defaultLibrary|Err§(_) => §variable.mutable|timing§.§method.mutable|error§(§namespace.crateRoot|serde_json§::§macro|json§!({"kind"§keyword.async|:§ "transport"})),
                _ => {}
            }
            §variable|response§
        })
    }
}
````

## Where the tokens come from

The marks in the third pane were not written by hand. `tools/lsp-marks` is a
small Neovim plugin that asks a language server for the semantic tokens of a
file and writes them as `§type.modifier|text§` marks. With the plugin on the
runtime path, in an editor where the server already runs with your own
settings:

```vim
:99,161LspMarksYank
```

copies that range, marked, to the clipboard. Without an editor, for building a
book in a script, the same code runs headless and starts the server itself:

```bash
nvim --headless -l tools/lsp-marks/headless.lua src/session.rs 99:161
```

Either way it leaves out what a grammar already colours (comments, strings,
numbers, punctuation) and modifiers that say little (`public`, `library`,
`reference`, `procMacro`).

The snippet is from the `m2-ls` language server. To show more of what a server
knows, it also has a `#[derive]`, a `#[tracing::instrument]` procedural macro on
`call`, an intra-doc link, and a doctest in its rustdoc, which rust-analyzer
highlights as injected Rust.

## And back: marks as highlighted text

`:LspMarksPreview` opens a split that shows a buffer written in marks as
highlighted code. It strips the markers, so tree-sitter parses clean code, and
colours each token with the highlight groups Neovim uses for semantic tokens
(`@lsp.type.interface.rust`, `@lsp.mod.mutable.rust`, and so on), so the colour
scheme you already have applies. The split follows every edit, which makes it a
quick way to colour a piece of text in Neovim without a language server at all.
