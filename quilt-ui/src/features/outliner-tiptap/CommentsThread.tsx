import { CommentRow } from '@features/comments/CommentRow'
import { buildCommentTree } from '@shared/utils/blockProperties'

// ──── CommentsThread ───────────────────────────────────────────────
// Recursive comment thread renderer. Comments are regular child blocks
// with `type: "comment"`; replies are nested comments of the same kind.

interface CommentsThreadProps {
  tree: ReturnType<typeof buildCommentTree>
  onResolve: (id: string) => void
  onReply: (id: string) => void
  onDelete?: (id: string) => void
  indent: number
}

export function CommentsThread({
  tree,
  onResolve,
  onReply,
  onDelete,
  indent,
}: CommentsThreadProps) {
  const totalResolved = countResolved(tree)

  return (
    <div
      data-testid={`comments-thread`}
      style={{
        marginLeft: `${indent * 24 + 32}px`,
        marginTop: 'var(--space-1)',
        padding: 'var(--space-2) var(--space-3)',
        background: 'var(--color-surface-subtle)',
        borderLeft: '2px solid var(--color-accent)',
        borderRadius: 'var(--radius-sm)',
      }}
    >
      <div
        style={{
          fontSize: '11px',
          fontWeight: 600,
          color: 'var(--color-text-muted)',
          marginBottom: 'var(--space-1)',
          display: 'flex',
          alignItems: 'center',
          gap: 'var(--space-1)',
        }}
      >
        <span aria-hidden="true">💬</span>
        <span>
          {tree.length} comment{tree.length > 1 ? 's' : ''}
        </span>
        {totalResolved > 0 && (
          <span style={{ color: 'var(--color-success)' }}>
            ({totalResolved} resolved)
          </span>
        )}
      </div>
      {tree.map(node => (
        <CommentThreadNode
          key={node.comment.id}
          node={node}
          onResolve={onResolve}
          onReply={onReply}
          onDelete={onDelete}
        />
      ))}
    </div>
  )
}

function CommentThreadNode({
  node,
  onResolve,
  onReply,
  onDelete,
  depth = 0,
}: {
  node: ReturnType<typeof buildCommentTree>[number]
  onResolve: (id: string) => void
  onReply: (id: string) => void
  onDelete?: (id: string) => void
  depth?: number
}) {
  return (
    <div>
      <CommentRow
        comment={node.comment}
        onResolve={onResolve}
        onReply={onReply}
        onDelete={onDelete}
        depth={depth}
      />
      {node.replies.length > 0 && (
        <div
          style={{
            marginLeft: 'var(--space-3)',
            borderLeft: '1px solid var(--color-border)',
            paddingLeft: 'var(--space-2)',
          }}
        >
          {node.replies.map(reply => (
            <CommentThreadNode
              key={reply.comment.id}
              node={reply}
              onResolve={onResolve}
              onReply={onReply}
              onDelete={onDelete}
              depth={depth + 1}
            />
          ))}
        </div>
      )}
    </div>
  )
}

function countResolved(
  tree: ReturnType<typeof buildCommentTree>,
): number {
  let count = 0
  for (const node of tree) {
    const resolved =
      String(
        node.comment.properties?.find(p => p.key === 'resolved')?.value ??
          'false',
      ) === 'true'
    if (resolved) count += 1
    count += countResolved(node.replies)
  }
  return count
}
