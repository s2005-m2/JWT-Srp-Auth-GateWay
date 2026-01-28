import * as srp from 'secure-remote-password/client';

export interface SrpRegistrationData {
  salt: string;
  verifier: string;
}

export interface SrpClientSession {
  clientSecretEphemeral: string;
  clientPublicEphemeral: string;
}

export function generateRegistrationData(
  email: string,
  password: string
): SrpRegistrationData {
  const salt = srp.generateSalt();
  const privateKey = srp.derivePrivateKey(salt, email, password);
  const verifier = srp.deriveVerifier(privateKey);
  return { salt, verifier };
}

export function createLoginSession(): SrpClientSession {
  const clientEphemeral = srp.generateEphemeral();
  return {
    clientSecretEphemeral: clientEphemeral.secret,
    clientPublicEphemeral: clientEphemeral.public,
  };
}

export function computeClientProof(
  email: string,
  password: string,
  salt: string,
  clientSecretEphemeral: string,
  serverPublicEphemeral: string
): string {
  const privateKey = srp.derivePrivateKey(salt, email, password);
  const session = srp.deriveSession(
    clientSecretEphemeral,
    serverPublicEphemeral,
    salt,
    email,
    privateKey
  );
  return session.proof;
}
