# NGREP
Searches the current working directory and any subdirectories for files whose names match certain text and outputs all lines in those files that match a regular expression. Includes line numbers. This project uses its own regular epression matching algorithm.

I made this project to help myself learn Rust. It's not menat to be optimal in any way. Nor is it meant to be fully featured.

## Executable:
ngrep.exe

## Usage:
ngrep [regex] [file_name_search] 

### regex
 a regular expression. This project supports the following matcher patterns: ., *, +, (), [], and simple escapes with '\\' (preceeding a character with '\\' will secape that character, but this project does not support "meta" escape sequences. For example: for this project, '\\s' excapes to 's' instead of matching for any whitespace character like it does elsewhere. '\\.' does excapes to '.' as you might expect--not to the wildcard matcher.).

### file_name_search:
the expression to match file names against. May be either the full file name or a string containing a single '*', which represents any number of characters of any value.

Exampels:
- 'abc.svg' matches any file name that is 'abc.svg'
- '*.txt' matches any file name ending with '.txt'
- 'data_bank_1*' matches any file name starting with 'data_bank_1'
- 'i*.html' matches any file name starting with 'i' and ending with '.html'

### Note:
<code>regex</code> is case sensitive. <code>file_name_search</code> is not.

## Example:

<pre>
Input:
ngrep .* *d.txt

Output:
C:\path\to\working\directory\test_files\f1st_inner_folder\bad.txt
LINE NUM  LINE TEXT
1         Baaaaaad
2         Not gooooood
3         Yep

C:\path\to\working\directory\test_files\f1st_inner_folder\good.txt
LINE NUM  LINE TEXT
1         Good for me
2         Bad for you
3         No one is happy
</pre>
