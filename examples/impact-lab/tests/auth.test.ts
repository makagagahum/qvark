import { postLogin } from '../src/routes/auth';

test('login creates a session', () => {
  expect(postLogin({ user: { id: 'u1' } })).toBeTruthy();
});
