export function before_transform() {
  return { minScore: 50 };
}

export function try_filter(item, ctx) {
  return item.score >= ctx.minScore;
}

export function try_map(item) {
  return {
    ...item,
    display_name: `${item.role.charAt(0).toUpperCase() + item.role.slice(1)}: ${item.name}`,
  };
}

export function after_transform(items) {
  return {
    version: "1.0",
    total_users: items.length,
    admins: items.filter(u => u.role === "admin").map(u => u.name),
    processed_at: new Date().toISOString(),
  };
}
