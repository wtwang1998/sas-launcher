import type { MessageKey } from '../i18n'

/**
 * Where an action's result belongs: next to the control that produced it, not
 * in a banner that scrolls out of view. Ambient problems keep their own place
 * at the top of the page and are not notices.
 */
export type NoticeTarget = 'form' | 'page' | 'dialog' | 'interface' | 'sas' | 'window'

export type Notice =
  | { tone: 'error'; target: NoticeTarget; error: unknown }
  | {
      tone: 'success'
      target: NoticeTarget
      key: MessageKey
      values?: Record<string, string | number>
    }
