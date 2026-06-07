// ─── CreateTemplateModal — sidebar template creation flow ────────
//
// Tests for the modal that lets users create a new template from the
// sidebar's "Plantillas" section without going through the terminal or
// REST API. The modal wires `api.createPage` (to create the
// `template/<name>` page) and `api.createBlock` (to attach the
// `card-shape` and `icon` properties to a seed block). All MOCK
// interactions are observable through the actual UI behaviour, never
// through internal spy counts.
//
// Spec: openspec/changes/quilt-template-management-ui/specs/
//       sidebar-template-create/spec.md
// Design: design.md §D7 (modal anchored in sidebar section), §D8
//         (creation flow: page + seed block with properties).

import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import toast from 'react-hot-toast'

// Imported AFTER the vi.mock declarations below so the hoisted mocks
// are in place when the module graph is evaluated.
import { CreateTemplateModal } from '../sections/CreateTemplateModal'

// ── Mocked dependencies ────────────────────────────────────────────
// We mock the api-client and toast. Toast is spied (not replaced) so
// the assertion is on the user-visible side effect, not on whether
// `toast.error` was called N times.

const mockCreatePage = vi.fn()
const mockCreateBlock = vi.fn()

vi.mock('@core/api-client', () => ({
  api: {
    createPage: (...args: unknown[]) => mockCreatePage(...args),
    createBlock: (...args: unknown[]) => mockCreateBlock(...args),
  },
}))

beforeEach(() => {
  mockCreatePage.mockReset()
  mockCreateBlock.mockReset()
})

// ── Helpers ────────────────────────────────────────────────────────

function renderModal(props: Partial<React.ComponentProps<typeof CreateTemplateModal>> = {}) {
  const onClose = props.onClose ?? vi.fn()
  const onCreated = props.onCreated ?? vi.fn()
  return {
    onClose,
    onCreated,
    ...render(
      <CreateTemplateModal isOpen={true} onClose={onClose} onCreated={onCreated} {...props} />,
    ),
  }
}

const CREATED_PAGE = {
  id: 'p-template-1',
  name: 'template/my-template',
  title: 'my-template',
  journal: false,
  journalDay: null,
  createdAt: '',
}

const CREATED_BLOCK = {
  id: 'b-seed-1',
  pageId: 'p-template-1',
  pageName: 'template/my-template',
  content: '',
  blockType: 'paragraph',
  marker: null,
  priority: null,
  parentId: null,
  order: 0,
  level: 0,
  collapsed: false,
  properties: [
    { key: 'card-shape', value: 'reference', type: 'string' },
    { key: 'icon', value: '🔗', type: 'string' },
  ],
  createdAt: '',
  updatedAt: '',
}

// ── Tests ──────────────────────────────────────────────────────────

describe('CreateTemplateModal — sidebar-template-create', () => {
  // ─── Rendering ───────────────────────────────────────────────

  it('renders nothing when isOpen=false (spec: Modal hidden until trigger)', () => {
    render(<CreateTemplateModal isOpen={false} onClose={vi.fn()} onCreated={vi.fn()} />)

    // The dialog title is a stable handle on the modal's presence.
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
    expect(screen.queryByText(/create template/i)).not.toBeInTheDocument()
  })

  it('renders the modal with name input, card-shape picker, icon picker, and submit button (spec: Modal contents)', () => {
    renderModal()

    // Dialog wrapper for accessibility (role=dialog + aria-modal).
    expect(screen.getByRole('dialog')).toBeInTheDocument()

    // Name input is the only textbox in the modal.
    const nameInput = screen.getByLabelText(/name/i)
    expect(nameInput).toBeInTheDocument()
    expect(nameInput).toHaveAttribute('placeholder')

    // All three card shapes are reachable through the picker. We assert
    // on the labels because they're the user-facing contract.
    expect(screen.getByText('Reference')).toBeInTheDocument()
    expect(screen.getByText('Content')).toBeInTheDocument()
    expect(screen.getByText('Inline')).toBeInTheDocument()

    // Icon picker is present (an input labelled "icon" or a preset row).
    expect(screen.getByLabelText('Icon', { exact: true })).toBeInTheDocument()

    // The submit button is the only one labelled "Create" (others are
    // "Cancel" / "Close").
    expect(
      screen.getByRole('button', { name: /^create$/i }),
    ).toBeInTheDocument()
    expect(
      screen.getByRole('button', { name: /cancel/i }),
    ).toBeInTheDocument()
  })

  // ─── Card-shape selection ────────────────────────────────────

  it('defaults the card-shape picker to "reference" (spec: Default shape)', () => {
    renderModal()

    // The "Reference" option is the default. We assert via the radio
    // group state, which is the only way the picker exposes selection.
    const referenceOption = screen.getByRole('radio', { name: /reference/i })
    expect(referenceOption).toBeChecked()
  })

  it('selecting a different card-shape updates the picker state (spec: Card shape selectable)', async () => {
    const user = userEvent.setup()
    renderModal()

    // The Content option is a radio in the same group.
    const contentOption = screen.getByRole('radio', { name: /content/i })
    await user.click(contentOption)

    // Selection moved — Reference is no longer checked.
    expect(contentOption).toBeChecked()
    expect(screen.getByRole('radio', { name: /reference/i })).not.toBeChecked()
  })

  // ─── Icon selection ──────────────────────────────────────────

  it('changes the icon input as the user types (spec: Icon input is editable)', async () => {
    const user = userEvent.setup()
    renderModal()

    const iconInput = screen.getByLabelText('Icon', { exact: true }) as HTMLInputElement
    await user.clear(iconInput)
    await user.type(iconInput, '📌')

    expect(iconInput.value).toBe('📌')
  })

  // ─── Validation ──────────────────────────────────────────────

  it('does NOT call createPage when the name is empty (spec: Validation — empty name rejected)', async () => {
    const user = userEvent.setup()
    renderModal()

    // The Create button is disabled until the name has at least one
    // non-whitespace character. This is the user-visible contract; no
    // implementation-detail assertion on disabled state alone — we
    // also verify the API is never called.
    const submit = screen.getByRole('button', { name: /^create$/i })
    expect(submit).toBeDisabled()
    await user.click(submit)

    expect(mockCreatePage).not.toHaveBeenCalled()
    expect(mockCreateBlock).not.toHaveBeenCalled()
  })

  it('does NOT call createPage when the name contains a `/` (spec: Validation — only flat names accepted)', async () => {
    const user = userEvent.setup()
    renderModal()

    const nameInput = screen.getByLabelText(/name/i)
    await user.type(nameInput, 'foo/bar')

    const submit = screen.getByRole('button', { name: /^create$/i })
    expect(submit).toBeDisabled()

    await user.click(submit)
    expect(mockCreatePage).not.toHaveBeenCalled()
  })

  // ─── Create flow ─────────────────────────────────────────────

  it('on submit creates a page named `template/<name>` and a seed block with card-shape + icon properties (spec: Creation flow)', async () => {
    const user = userEvent.setup()
    mockCreatePage.mockResolvedValue(CREATED_PAGE)
    mockCreateBlock.mockResolvedValue(CREATED_BLOCK)
    const { onClose, onCreated } = renderModal()

    // Fill the name and pick a non-default shape + icon.
    const nameInput = screen.getByLabelText(/name/i)
    await user.type(nameInput, 'my-template')

    const contentOption = screen.getByRole('radio', { name: /content/i })
    await user.click(contentOption)

    const iconInput = screen.getByLabelText('Icon', { exact: true }) as HTMLInputElement
    await user.clear(iconInput)
    await user.type(iconInput, '📄')

    // Submit.
    await user.click(screen.getByRole('button', { name: /^create$/i }))

    // The page is created with the `template/` prefix.
    await waitFor(() => {
      expect(mockCreatePage).toHaveBeenCalledWith({
        name: 'template/my-template',
      })
    })

    // The seed block is created on that page with both properties.
    await waitFor(() => {
      expect(mockCreateBlock).toHaveBeenCalledWith(
        expect.objectContaining({
          pageName: 'template/my-template',
          properties: expect.objectContaining({
            'card-shape': 'content',
            icon: '📄',
          }),
        }),
      )
    })

    // The modal notifies the parent so the section can refresh, and
    // closes itself.
    expect(onCreated).toHaveBeenCalledWith('template/my-template')
    expect(onClose).toHaveBeenCalledTimes(1)
  })

  it('shows a success toast and surfaces it to the user (spec: Success feedback)', async () => {
    const user = userEvent.setup()
    const successSpy = vi.spyOn(toast, 'success').mockImplementation(() => '')
    mockCreatePage.mockResolvedValue(CREATED_PAGE)
    mockCreateBlock.mockResolvedValue(CREATED_BLOCK)

    renderModal()
    await user.type(screen.getByLabelText(/name/i), 'my-template')
    await user.click(screen.getByRole('button', { name: /^create$/i }))

    await waitFor(() => {
      expect(successSpy).toHaveBeenCalledWith(
        expect.stringMatching(/template.*created/i),
      )
    })

    successSpy.mockRestore()
  })

  it('shows an error toast and keeps the modal open when createPage fails (spec: Error feedback)', async () => {
    const user = userEvent.setup()
    const errorSpy = vi.spyOn(toast, 'error').mockImplementation(() => '')
    mockCreatePage.mockRejectedValue(new Error('network down'))

    const { onClose } = renderModal()
    await user.type(screen.getByLabelText(/name/i), 'my-template')
    await user.click(screen.getByRole('button', { name: /^create$/i }))

    await waitFor(() => {
      expect(errorSpy).toHaveBeenCalledWith(
        expect.stringMatching(/failed to create template/i),
      )
    })

    // Modal stays open so the user can retry.
    expect(onClose).not.toHaveBeenCalled()
    errorSpy.mockRestore()
  })

  it('does not create the seed block if the page creation fails (spec: No partial writes)', async () => {
    const user = userEvent.setup()
    mockCreatePage.mockRejectedValue(new Error('forbidden'))

    renderModal()
    await user.type(screen.getByLabelText(/name/i), 'my-template')
    await user.click(screen.getByRole('button', { name: /^create$/i }))

    await waitFor(() => {
      expect(mockCreatePage).toHaveBeenCalledTimes(1)
    })
    // No block is created if the page failed — otherwise we'd leave an
    // orphan page with no template metadata.
    expect(mockCreateBlock).not.toHaveBeenCalled()
  })

  // ─── Cancel / dismiss ────────────────────────────────────────

  it('calls onClose when the Cancel button is clicked (spec: Cancel button)', async () => {
    const user = userEvent.setup()
    const { onClose } = renderModal()

    await user.click(screen.getByRole('button', { name: /cancel/i }))

    expect(onClose).toHaveBeenCalledTimes(1)
    expect(mockCreatePage).not.toHaveBeenCalled()
  })

  it('closes the modal when the user presses Escape (spec: Escape dismisses)', async () => {
    const user = userEvent.setup()
    const { onClose } = renderModal()

    // Escape is bound at the dialog level. The name input is focused
    // first; firing Escape there should still close the modal.
    await user.keyboard('{Escape}')

    expect(onClose).toHaveBeenCalledTimes(1)
  })

  it('does not call createPage when the modal is closed before submit (spec: No side effects on dismiss)', async () => {
    const user = userEvent.setup()
    const { onClose } = renderModal()

    // Type a name but cancel without submitting.
    await user.type(screen.getByLabelText(/name/i), 'orphan')
    await user.click(screen.getByRole('button', { name: /cancel/i }))

    expect(mockCreatePage).not.toHaveBeenCalled()
    expect(onClose).toHaveBeenCalledTimes(1)
  })
})
