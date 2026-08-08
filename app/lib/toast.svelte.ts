/**
 * Transient notifications. Modelled on plyr.fm's toast store, trimmed to what
 * this app actually raises: a confirmation, or something that went wrong.
 */

export type ToastType = "success" | "error";

export interface Toast {
  id: string;
  message: string;
  type: ToastType;
}

class ToastState {
  toasts = $state<Toast[]>([]);

  private add(message: string, type: ToastType, duration: number): string {
    const id = crypto.randomUUID();
    // newest first: the container stacks upward from the bottom
    this.toasts = [{ id, message, type }, ...this.toasts];
    if (duration > 0) setTimeout(() => this.dismiss(id), duration);
    return id;
  }

  dismiss(id: string): void {
    this.toasts = this.toasts.filter((t) => t.id !== id);
  }

  success(message: string, duration = 2500): string {
    return this.add(message, "success", duration);
  }

  /** Longer, because a failure is worth reading and may need acting on. */
  error(message: string, duration = 5000): string {
    return this.add(message, "error", duration);
  }
}

export const toast = new ToastState();
