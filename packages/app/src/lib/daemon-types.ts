/** Wire DTOs not yet in typeshare-generated protocol types. */

export interface SearchHit {
  conversation_id: string;
  title: string;
  snippet: string;
  match_field: string;
}

export interface ImportWarning {
  kind: string;
  detail: string;
}

export interface ImportResult {
  conversation: {
    id: string;
    title: string;
    active_harness_id?: string;
    model_id?: string;
    transcript_json: string;
  };
  warnings: ImportWarning[];
}

export interface VaultIssue {
  kind: string;
  conversation_id?: string;
  path?: string;
  reason?: string;
  winner_path?: string;
  loser_paths?: string[];
  detail: string;
}

export interface WorkdirFile {
  relative_path: string;
  size: number;
  mime_type?: string;
  modified_at: number;
}
