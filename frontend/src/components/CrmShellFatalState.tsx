type CrmShellFatalStateProps = {
  message: string;
};

export function CrmShellFatalState({ message }: CrmShellFatalStateProps) {
  return (
    <main className="crm-foundation-shell">
      <section className="crm-foundation-card">
        <p className="crm-foundation-eyebrow">CRM</p>
        <h1>Не удалось открыть React-оболочку CRM</h1>
        <p className="mb-0 text-secondary">{message}</p>
      </section>
    </main>
  );
}
