import { ShellFatalState } from "@pushkind/frontend-shell/ShellFatalState";

type CrmShellFatalStateProps = {
  message: string;
};

export function CrmShellFatalState({ message }: CrmShellFatalStateProps) {
  return (
    <ShellFatalState
      message={message}
      serviceLabel="CRM"
      title="Не удалось открыть React-оболочку CRM"
      shellClassName="crm-foundation-shell"
      cardClassName="crm-foundation-card"
      eyebrowClassName="crm-foundation-eyebrow"
    />
  );
}
