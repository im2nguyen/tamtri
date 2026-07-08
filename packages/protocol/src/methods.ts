/**
 * Wire method registry. Mirrors the `method` module in core/src/protocol/mod.rs
 * one-to-one. JSON-RPC correlates a request with its response by `id`, so there
 * is no `.request`/`.response` suffix; the pairing is structural.
 *
 * typeshare generates the data types but not these string constants, so this
 * list is kept in sync with the Rust source by hand (the names are stable).
 */
export const method = {
  // Handshake
  HELLO: "hello",
  // Streaming push (daemon -> client); params are an EventNotification.
  EVENT: "event",

  // Harness roster / models
  AGENTS_LIST: "agents.list",
  AGENTS_MODELS: "agents.models",

  // Conversations
  CONVERSATION_LIST: "conversation.list",
  CONVERSATION_LOAD: "conversation.load",
  CONVERSATION_CREATE: "conversation.create",
  CONVERSATION_FORK: "conversation.fork",
  CONVERSATION_DELETE: "conversation.delete",
  CONVERSATION_SEND_MESSAGE: "conversation.send_message",
  CONVERSATION_FOLDER_PATH: "conversation.folder_path",
  CONVERSATION_EXPORT_BUNDLE: "conversation.export_bundle",
  CONVERSATION_IMPORT: "conversation.import",

  // Run control
  RUN_CANCEL: "run.cancel",
  PERMISSION_RESPOND: "permission.respond",
  ELICITATION_RESPOND: "elicitation.respond",
  TASK_CANCEL: "task.cancel",

  // Roots
  ROOTS_LIST: "roots.list",
  ROOTS_ATTACH: "roots.attach",
  ROOTS_REMOVE: "roots.remove",
  ROOTS_SYNC_RUNTIME: "roots.sync_runtime",

  // Workdir / attachments / artifacts
  WORKDIR_COPY_FILE: "workdir.copy_file",
  WORKDIR_LIST_FILES: "workdir.list_files",
  WORKDIR_PATH: "workdir.path",
  WORKDIR_READ_FILE: "workdir.read_file",
  ATTACHMENT_READ_VERIFIED: "attachment.read_verified",
  ATTACHMENT_VERIFIED_PATH: "attachment.verified_path",
  ARTIFACT_VERIFY_INLINE: "artifact.verify_inline",
  ARTIFACT_LOG_NAVIGATION_BLOCKED: "artifact.log_navigation_blocked",

  // MCP Apps
  APP_RESOLVE_TEMPLATE: "app.resolve_template",
  APP_SUBMIT_BRIDGE_REQUEST: "app.submit_bridge_request",
  APP_RESPOND_BRIDGE_CONSENT: "app.respond_bridge_consent",
  APP_LOG_NAVIGATION_BLOCKED: "app.log_navigation_blocked",
  APP_BRIDGE_BOOTSTRAP_SCRIPT: "app.bridge_bootstrap_script",
  APP_PREPARE_QUIT: "app.prepare_quit",

  // Gateway (MCP servers + credentials + oauth)
  GATEWAY_LIST_SERVERS: "gateway.list_servers",
  GATEWAY_REFRESH_CAPABILITIES: "gateway.refresh_capabilities",
  GATEWAY_LIST_TOOLS: "gateway.list_tools",
  GATEWAY_GET_SETTINGS: "gateway.get_settings",
  GATEWAY_SET_DEFAULT_TIMEOUT: "gateway.set_default_timeout",
  GATEWAY_SAVE_SERVERS: "gateway.save_servers",
  GATEWAY_SET_CREDENTIAL: "gateway.set_credential",
  GATEWAY_EXPORT_CREDENTIAL: "gateway.export_credential",
  GATEWAY_START_OAUTH: "gateway.start_oauth",
  GATEWAY_COMPLETE_OAUTH: "gateway.complete_oauth",

  // Search / health / vault / diagnostics
  SEARCH_CONVERSATIONS: "search.conversations",
  SEARCH_SCOPE_MESSAGE: "search.scope_message",
  HARNESS_HEALTH_LIST: "harness.health_list",
  HARNESS_HEALTH_CHECKLIST: "harness.health_checklist",
  VAULT_ISSUES: "vault.issues",
  VAULT_PATH: "vault.path",
  DIAGNOSTICS_WRITE_BUNDLE: "diagnostics.write_bundle",

  // Relay (remote access)
  RELAY_PAIRING_OFFER: "relay.pairing_offer",

  // Native session import
  SESSIONS_LIST_NATIVE: "sessions.list_native",
  SESSIONS_IMPORT: "sessions.import",
} as const;

export type MethodName = (typeof method)[keyof typeof method];
