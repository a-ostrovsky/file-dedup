# Create test directory
$testDir = "test_files"
New-Item -ItemType Directory -Force -Path $testDir

# Create files with same content (duplicates)
$content1 = "This is a test file with some content that will be duplicated."
$content1 | Out-File -FilePath "$testDir\duplicate1.txt"
$content1 | Out-File -FilePath "$testDir\duplicate2.txt"
$content1 | Out-File -FilePath "$testDir\duplicate3.txt"

# Create files with same size but different content
$content2 = "111"
$content3 = "222"
$content2 | Out-File -FilePath "$testDir\same_size1.txt"
$content3 | Out-File -FilePath "$testDir\same_size2.txt"

# Create empty files
"" | Out-File -FilePath "$testDir\empty1.txt"
"" | Out-File -FilePath "$testDir\empty2.txt"

# Create files with different extensions but same content
$content4 = "This content will be in files with different extensions."
$content4 | Out-File -FilePath "$testDir\same_content.txt"
$content4 | Out-File -FilePath "$testDir\same_content.md"
$content4 | Out-File -FilePath "$testDir\same_content.log"

# Create a larger file
$largeContent = "This is a larger file. " * 1000
$largeContent | Out-File -FilePath "$testDir\large_file.txt"

Write-Host "Test files have been created in the '$testDir' directory" 