import { useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '../components/ui/Card';
import { Button } from '../components/ui/Button';
import { Input } from '../components/ui/Input';
import { generateRegistrationData, createLoginSession, computeClientProof } from '../lib/srp';

interface ApiResponse {
  data: unknown;
  status: number;
  ok: boolean;
}

function ResponseDisplay({ response }: { response: ApiResponse | null }) {
  if (!response) return null;
  return (
    <pre className={`mt-4 p-3 rounded text-sm overflow-auto max-h-48 ${
      response.ok ? 'bg-green-50 text-green-800' : 'bg-red-50 text-red-800'
    }`}>
      {JSON.stringify(response.data, null, 2)}
    </pre>
  );
}

const GATEWAY_URL = 'http://localhost:8080';

export default function AuthTest() {
  const [registerEmail, setRegisterEmail] = useState('');
  const [registerRes, setRegisterRes] = useState<ApiResponse | null>(null);
  const [registerLoading, setRegisterLoading] = useState(false);

  const [verifyEmail, setVerifyEmail] = useState('');
  const [verifyCode, setVerifyCode] = useState('');
  const [verifyPassword, setVerifyPassword] = useState('');
  const [verifyRes, setVerifyRes] = useState<ApiResponse | null>(null);
  const [verifyLoading, setVerifyLoading] = useState(false);

  const [loginEmail, setLoginEmail] = useState('');
  const [loginPassword, setLoginPassword] = useState('');
  const [loginRes, setLoginRes] = useState<ApiResponse | null>(null);
  const [loginLoading, setLoginLoading] = useState(false);

  const [refreshToken, setRefreshToken] = useState('');
  const [refreshRes, setRefreshRes] = useState<ApiResponse | null>(null);
  const [refreshLoading, setRefreshLoading] = useState(false);

  const callApi = async (
    url: string,
    body: object,
    setRes: (r: ApiResponse) => void,
    setLoading: (l: boolean) => void
  ) => {
    setLoading(true);
    try {
      const res = await fetch(url, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });
      const data = await res.json();
      setRes({ data, status: res.status, ok: res.ok });
    } catch (err) {
      setRes({ data: { error: String(err) }, status: 0, ok: false });
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-3xl font-bold tracking-tight">Auth API Test</h2>
        <p className="text-slate-500 mt-2">Test user authentication endpoints</p>
      </div>

      <div className="grid gap-6 md:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle className="text-lg">POST /auth/register</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              <Input
                placeholder="Email"
                value={registerEmail}
                onChange={(e) => setRegisterEmail(e.target.value)}
              />
              <Button
                className="w-full"
                isLoading={registerLoading}
                onClick={() => callApi(`${GATEWAY_URL}/auth/register`, { email: registerEmail }, setRegisterRes, setRegisterLoading)}
              >
                Send Verification Code
              </Button>
            </div>
            <ResponseDisplay response={registerRes} />
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-lg">POST /auth/register/verify</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              <Input
                placeholder="Email"
                value={verifyEmail}
                onChange={(e) => setVerifyEmail(e.target.value)}
              />
              <Input
                placeholder="Verification Code"
                value={verifyCode}
                onChange={(e) => setVerifyCode(e.target.value)}
              />
              <Input
                type="password"
                placeholder="Password"
                value={verifyPassword}
                onChange={(e) => setVerifyPassword(e.target.value)}
              />
              <Button
                className="w-full"
                isLoading={verifyLoading}
                onClick={() => {
                  const { salt, verifier } = generateRegistrationData(verifyEmail, verifyPassword);
                  callApi(`${GATEWAY_URL}/auth/register/verify`, {
                    email: verifyEmail,
                    code: verifyCode,
                    salt,
                    verifier,
                  }, setVerifyRes, setVerifyLoading);
                }}
              >
                Verify & Create Account (SRP)
              </Button>
            </div>
            <ResponseDisplay response={verifyRes} />
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-lg">SRP Login (2-step)</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              <Input
                placeholder="Email"
                value={loginEmail}
                onChange={(e) => setLoginEmail(e.target.value)}
              />
              <Input
                type="password"
                placeholder="Password"
                value={loginPassword}
                onChange={(e) => setLoginPassword(e.target.value)}
              />
              <Button
                className="w-full"
                isLoading={loginLoading}
                onClick={async () => {
                  setLoginLoading(true);
                  try {
                    const session = createLoginSession();
                    const initRes = await fetch(`${GATEWAY_URL}/auth/login/init`, {
                      method: 'POST',
                      headers: { 'Content-Type': 'application/json' },
                      body: JSON.stringify({
                        email: loginEmail,
                        client_public: session.clientPublicEphemeral,
                      }),
                    });
                    const initData = await initRes.json();
                    if (!initRes.ok) {
                      setLoginRes({ data: initData, status: initRes.status, ok: false });
                      return;
                    }
                    const clientProof = computeClientProof(
                      loginEmail,
                      loginPassword,
                      initData.salt,
                      session.clientSecretEphemeral,
                      initData.server_public
                    );
                    const verifyRes = await fetch(`${GATEWAY_URL}/auth/login/verify`, {
                      method: 'POST',
                      headers: { 'Content-Type': 'application/json' },
                      body: JSON.stringify({
                        session_id: initData.session_id,
                        client_proof: clientProof,
                      }),
                    });
                    const verifyData = await verifyRes.json();
                    setLoginRes({ data: verifyData, status: verifyRes.status, ok: verifyRes.ok });
                  } catch (err) {
                    setLoginRes({ data: { error: String(err) }, status: 0, ok: false });
                  } finally {
                    setLoginLoading(false);
                  }
                }}
              >
                Login (SRP)
              </Button>
            </div>
            <ResponseDisplay response={loginRes} />
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-lg">POST /auth/refresh</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              <Input
                placeholder="Refresh Token"
                value={refreshToken}
                onChange={(e) => setRefreshToken(e.target.value)}
              />
              <Button
                className="w-full"
                isLoading={refreshLoading}
                onClick={() => callApi(`${GATEWAY_URL}/auth/refresh`, {
                  refresh_token: refreshToken,
                }, setRefreshRes, setRefreshLoading)}
              >
                Refresh Token
              </Button>
            </div>
            <ResponseDisplay response={refreshRes} />
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
