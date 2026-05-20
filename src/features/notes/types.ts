export interface NoteMetadata {
  id: string;
  title: string;
  fileName: string;
  category: string;
  pinned: boolean;
  createdAt: string;
  updatedAt: string;
  wordCount: number;
  preview: string;
}

export interface Note extends Omit<NoteMetadata, "preview"> {
  content: string;
}

export interface SaveNoteRequest {
  title: string;
  content: string;
  category: string;
}

export interface ExternalFile {
  id: string;
  title: string;
  filePath: string;
}
