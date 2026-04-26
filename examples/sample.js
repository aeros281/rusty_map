export default function transform(data) {
  const admins = data.users.filter((u) => u.role === "admin").map((u) => u.name);

  return {
    version: data.version,
    total_users: data.users.length,
    admins,
    processed_at: new Date().toISOString(),
  };
}
