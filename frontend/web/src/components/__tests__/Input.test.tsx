/**
 * Input 组件测试
 */
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { Input, Textarea } from '../Input';

describe('Input', () => {
  it('renders basic input', () => {
    render(<Input placeholder="Enter text" />);
    expect(screen.getByPlaceholderText('Enter text')).toBeInTheDocument();
  });

  it('renders with label', () => {
    render(<Input label="Email" />);
    expect(screen.getByText('Email')).toBeInTheDocument();
  });

  it('renders with error message', () => {
    render(<Input error="This field is required" />);
    expect(screen.getByText(/This field is required/)).toBeInTheDocument();
  });

  it('renders with hint message', () => {
    render(<Input hint="Enter your email" />);
    expect(screen.getByText('Enter your email')).toBeInTheDocument();
  });

  it('does not show hint when error is present', () => {
    render(<Input error="Error" hint="Hint" />);
    expect(screen.getByText(/Error/)).toBeInTheDocument();
    expect(screen.queryByText('Hint')).not.toBeInTheDocument();
  });

  it('applies error class when error is present', () => {
    render(<Input error="Error message" />);
    const input = screen.getByRole('textbox');
    expect(input.className).toContain('input-error');
  });

  it('handles value changes', () => {
    const handleChange = vi.fn();
    render(<Input onChange={handleChange} />);
    fireEvent.change(screen.getByRole('textbox'), { target: { value: 'test' } });
    expect(handleChange).toHaveBeenCalledTimes(1);
  });

  it('renders with icon', () => {
    render(<Input icon={<span>📧</span>} />);
    expect(screen.getByText('📧')).toBeInTheDocument();
  });

  it('applies custom className', () => {
    const { container } = render(<Input className="custom" />);
    expect(container.firstChild).toHaveClass('custom');
  });

  it('uses provided id', () => {
    render(<Input id="my-input" label="Test" />);
    const input = screen.getByRole('textbox');
    expect(input).toHaveAttribute('id', 'my-input');
  });

  it('generates random id when not provided', () => {
    render(<Input label="Test" />);
    const input = screen.getByRole('textbox');
    expect(input.id).toMatch(/^input-/);
  });

  it('associates label with input via htmlFor', () => {
    render(<Input id="email-input" label="Email" />);
    const label = screen.getByText('Email');
    expect(label).toHaveAttribute('for', 'email-input');
  });

  it('forwards HTML input attributes', () => {
    render(<Input type="email" placeholder="test@example.com" required />);
    const input = screen.getByRole('textbox');
    expect(input).toHaveAttribute('type', 'email');
    expect(input).toHaveAttribute('placeholder', 'test@example.com');
    expect(input).toBeRequired();
  });

  it('can be disabled', () => {
    render(<Input disabled />);
    expect(screen.getByRole('textbox')).toBeDisabled();
  });
});

describe('Textarea', () => {
  it('renders basic textarea', () => {
    render(<Textarea placeholder="Enter text" />);
    expect(screen.getByPlaceholderText('Enter text')).toBeInTheDocument();
  });

  it('renders with label', () => {
    render(<Textarea label="Description" />);
    expect(screen.getByText('Description')).toBeInTheDocument();
  });

  it('renders with error message', () => {
    render(<Textarea error="Too long" />);
    expect(screen.getByText(/Too long/)).toBeInTheDocument();
  });

  it('renders with hint message', () => {
    render(<Textarea hint="Max 500 chars" />);
    expect(screen.getByText('Max 500 chars')).toBeInTheDocument();
  });

  it('handles value changes', () => {
    const handleChange = vi.fn();
    render(<Textarea onChange={handleChange} />);
    fireEvent.change(screen.getByRole('textbox'), { target: { value: 'test' } });
    expect(handleChange).toHaveBeenCalledTimes(1);
  });

  it('uses provided id', () => {
    render(<Textarea id="my-textarea" />);
    expect(screen.getByRole('textbox')).toHaveAttribute('id', 'my-textarea');
  });

  it('generates random id when not provided', () => {
    render(<Textarea />);
    expect(screen.getByRole('textbox').id).toMatch(/^textarea-/);
  });
});
