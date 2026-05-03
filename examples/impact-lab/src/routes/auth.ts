import { issueSession } from '../services/session';
import { auditEvent } from '../services/audit';

export function postLogin(req) {
  const session = issueSession(req.user.id);
  auditEvent('login');
  return session;
}
