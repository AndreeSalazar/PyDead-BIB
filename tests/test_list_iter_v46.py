def main() -> int:
    my_list = [10, 20, 30]
    
    # Test simple list iteration
    for x in my_list:
        print(x)
        
    # Test string list iteration
    str_list = ["apple", "banana", "cherry"]
    for s in str_list:
        print(s)
        
    # Test list returned from split()
    words = "hello world pydead bib".split(" ")
    for w in words:
        print(w)
        
    return 0

main()
