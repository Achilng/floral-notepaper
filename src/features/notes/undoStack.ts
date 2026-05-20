export interface ListCommand {
  execute(): Promise<void>;
  undo(): Promise<void>;
  description: string;
}

const MAX_STACK_SIZE = 20;

export class ListUndoStack {
  private stack: ListCommand[] = [];

  push(command: ListCommand): void {
    this.stack.push(command);
    if (this.stack.length > MAX_STACK_SIZE) {
      this.stack.shift();
    }
  }

  async undo(): Promise<string | null> {
    const command = this.stack.pop();
    if (!command) return null;
    await command.undo();
    return command.description;
  }

  clear(): void {
    this.stack.length = 0;
  }

  get size(): number {
    return this.stack.length;
  }
}
