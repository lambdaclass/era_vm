object "Simple_add" {
    code {
        let a := 1
        let b := 2

        let c := add(a, b)
        sstore(0, c)
    }
    object "Simple_add_deployed" {
        code {
            let a := 1
            let b := 2

            let c := add(a, b)
            sstore(0, c)
        }
    }
}
