pragma solidity ^0.8.0;

contract benchmark {
    uint256 value;
    constructor() {
      value = fib(370);
    }

    function get_calculation() public view returns (uint256) {
        return value;
    }

    function fib(uint n) internal pure returns(uint b) { 
        if (n == 0) {
            return 0;   
        }
        uint a = 1;
        b = 1;
        for (uint i = 2; i < n; i++) {
            uint c = a + b;
            a = b;
            b = c;
        }
        return b;
    }
}
