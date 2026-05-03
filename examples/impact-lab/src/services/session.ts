export function issueSession(userId) {
  return {
    userId,
    expiresIn: 3600,
  };
}
