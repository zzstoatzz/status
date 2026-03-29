export function relativeTime(dateStr: string): string {
  const date = new Date(dateStr);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMs / 3600000);
  const diffDays = Math.floor(diffMs / 86400000);

  if (diffMs < 30000) return "just now";
  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffHours < 24) {
    const remainingMins = diffMins % 60;
    return remainingMins === 0
      ? `${diffHours}h ago`
      : `${diffHours}h ${remainingMins}m ago`;
  }
  if (diffDays < 7) {
    const remainingHours = diffHours % 24;
    return remainingHours === 0
      ? `${diffDays}d ago`
      : `${diffDays}d ${remainingHours}h ago`;
  }

  const timeStr = date
    .toLocaleTimeString("en-US", { hour: "numeric", minute: "2-digit", hour12: true })
    .toLowerCase();
  if (date.getFullYear() === now.getFullYear()) {
    return (
      date.toLocaleDateString("en-US", { month: "short", day: "numeric" }) +
      ", " +
      timeStr
    );
  }
  return (
    date.toLocaleDateString("en-US", {
      month: "short",
      day: "numeric",
      year: "numeric",
    }) +
    ", " +
    timeStr
  );
}

export function formatExpiration(dateStr: string): string {
  const date = new Date(dateStr);
  const now = new Date();
  const diffMs = date.getTime() - now.getTime();

  if (diffMs <= 0) {
    const agoMs = Math.abs(diffMs);
    const agoMins = Math.floor(agoMs / 60000);
    if (agoMins < 1) return "expired";
    if (agoMins < 60) return `expired ${agoMins}m ago`;
    const agoHours = Math.floor(agoMs / 3600000);
    if (agoHours < 24) return `expired ${agoHours}h ago`;
    const agoDays = Math.floor(agoMs / 86400000);
    return `expired ${agoDays}d ago`;
  }

  return `clears ${relativeTimeFuture(dateStr)}`;
}

export function relativeTimeFuture(dateStr: string): string {
  const date = new Date(dateStr);
  const now = new Date();
  const diffMs = date.getTime() - now.getTime();

  if (diffMs <= 0) return "now";

  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMs / 3600000);
  const diffDays = Math.floor(diffMs / 86400000);

  if (diffMins < 1) return "in less than a minute";
  if (diffMins < 60) return `in ${diffMins}m`;
  if (diffHours < 24) {
    const remainingMins = diffMins % 60;
    return remainingMins === 0
      ? `in ${diffHours}h`
      : `in ${diffHours}h ${remainingMins}m`;
  }
  if (diffDays < 7) {
    const remainingHours = diffHours % 24;
    return remainingHours === 0
      ? `in ${diffDays}d`
      : `in ${diffDays}d ${remainingHours}h`;
  }

  const timeStr = date
    .toLocaleTimeString("en-US", { hour: "numeric", minute: "2-digit", hour12: true })
    .toLowerCase();
  if (date.getFullYear() === now.getFullYear()) {
    return (
      date.toLocaleDateString("en-US", { month: "short", day: "numeric" }) +
      ", " +
      timeStr
    );
  }
  return (
    date.toLocaleDateString("en-US", {
      month: "short",
      day: "numeric",
      year: "numeric",
    }) +
    ", " +
    timeStr
  );
}
