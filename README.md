# ☑️ Todome

（日本語版は[こちら](./README-ja.md)）

Todome is a notation developed for making and editing to-do lists.
It is inspired by [Todo.txt](http://todotxt.org), and in fact some of the todome notation is similar to that of Todo.txt.

## Support Tools

Todome provides several tools to help you manage your tasks. By using these, you can achieve more effective and efficient task management than simply editing text.

* [tree-sitter-todome](https://github.com/monaqa/tree-sitter-todome): a todome parser written using [Tree-sitter](https://tree-sitter.github.io/tree-sitter/).
* todome CLI (in this repository): CLI tool for formatting todome notation, etc.
* todome-language-server (in this repository): language server for helping edit your todome files.
  * Complete category names and tag names
  * Highlight overdue tasks

## Todome Notation

Tasks in todome notation are written in files with the extension `.todome`.

An overview of the todome notation is shown in the figure below:

![todome-notation.png](https://user-images.githubusercontent.com/48883418/140614506-03fd700d-2791-44a2-baa5-0b1f9590c597.png)

### Basic Tasks

The basic idea of todome is to write one task per line.
Like Todo.txt, you can just string text together and it will be treated as a task.

```
Return books at the library
Buy milk
Have a meeting with xxx on the phone
Reply to email from xxx
```

### Status (To Do, Doing, Done, Cancelled)

You can indicate the status of a task by prefixing it with a symbol such as `-` or `*`.

```
Return books at the library              # Status: To Do (default)
- Buy milk                               # Status: Done
= Have a meeting with xxx on the phone   # Status: Cancelled
Reply to email from xxx
```

There are four types of symbols that represent states.

|Symbol|Status   |Note   |
|------|---------|-------|
|`+`   |To Do    |Default|
|`*`   |Doing    |       |
|`-`   |Done     |       |
|`=`   |Caneclled|       |

### Meta-Information (Priority, Due Date, and Category)

Each task can contain meta-information such as priority, due date, and category.
The meta-information is written between the status and the task body.

```
(B) (2021-11-13) Return books at the library      # Priority: B, Due date: Nov. 13, 2021
- [shopping] Buy milk                             # Status: Done, Category: shopping
[work] Have a meeting with xxx on the phone       # Category: work
- (A) [work] [Project X] Reply to email from xxx  # Status: Done, Priority: A, Category: work and "Project X"
```

* Priority: `(A)`, `(B)`, `(C)`, ..., `(Z)`
  * indicated by round parentheses and a single uppercase alphabet character
  * `(A)` represents the highest priority, `(Z)` the lowest
* Due date: `(2021-11-06)`, etc.
  * indicated by round parentheses and a string in the format `YYYY-mm-dd`
* Category: `[work]`, `[Project X]`, `[About xxx's email]`, etc.
  * indicated by square brackets and a non-empty string
  * category names (the contents of square brackets) can contain almost any character, but must not contain characters such as `[`, `]`, `#`, and line feeds.

### Subtasks

In todome, indentation can be used to indicate the hierarchical structure of tasks.
Indentation must be done using the **TAB character** (indenting with spaces will simply be ignored).

```
# Use TAB character to indent
Shopping
	milk
	6 eggs
```

Subtask itself can contain meta-information.

```
Shopping
	- (A) milk   # Status: Done, Priority: A
	(C) 6 eggs   # Status: To Do, Priority: C
```

If you write the following, the attributes (status and meta-information) of the parent task will be inherited, and the tasks "Shopping", "milk", and "6 eggs" will all be treated as done.
The attributes can be overridden.


```
- Shopping
	(A) milk     # Status: Done, Priority: A
	(C) 6 eggs   # Status: Done, Priority: C
```

Subtasks can be nested.

```
shopping
	- Groceries
		milk
		(C) 6 eggs
	Daily necessities
		Dishwashing detergent
```

Subtasks can be used to describe detailed information about the parent task or to write notes on the progress of the parent task, as well as to break down the parent task. You can write them in any way you like as long as you don't use incorrect syntax.

```
Survey of papers about XXX
	Title: xxxxxx
	URL: https://...
	Sections
		- Introduction
		- Conventional method
		* Proposed method
		Experiment
```

### Headers

If you want to include multiple tasks in the same category, it is useful to use headers to organize tasks.
By writing one or more attributes and indenting the task with a TAB character on the next line, the attributes you just wrote will be reflected in all indented tasks.

```
(2021-11-13)
	(B) Return books at the library

[shopping]
	- Buy milk
	6 eggs

(A) [work]
	Have a meeting with xxx on the phone
	- Reply to email from xxx
```

This is equivalent for the following:

```
(2021-11-13) (B) Return books at the library

- [shopping] Buy milk
[shopping] 6 eggs

(A) [work] Have a meeting with xxx on the phone
- (A) [work] Reply to email from xxx
```

Although headers are similar to subtasks, there are some differences:
* A header line itself is not treated as a task.
* Each child element of the header is treated as an independent task.

### Tags

Each task body can contain some tags. A sequence of alphanumeric characters starting with `@` (`@[a-zA-Z0-9][a-zA-Z0-9_-]*` in regexp) is considered as a tag.

```
- (A) [work] Reply to @email from xxx
```

### Comments

`#` is treated as the start of an inline comment.
You can create a line that consists only of a comment. Comments are not treated as a task or a task body.



