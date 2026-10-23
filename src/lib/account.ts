// Typed wrappers around the Rust commands in src-tauri/src/account.rs.
// Keep AccountStatus in sync with the Rust enum of the same name.

import { invoke } from '@tauri-apps/api/core'

export type AccountStatus =
  | { state: 'no_config'; path: string; message: string }
  | { state: 'signed_out' }
  | { state: 'session_expired' }
  | { state: 'not_linked' }
  | { state: 'offline'; message: string }
  | { state: 'keyring_unavailable'; message: string }
  | { state: 'signed_in'; missing_scopes: string[] }
  | { state: 'error'; message: string }

export const accountStatus = () => invoke<AccountStatus>('account_status')
export const signIn = () => invoke<AccountStatus>('sign_in')
export const signOut = () => invoke<AccountStatus>('sign_out')
