import { gql } from 'apollo-server-express'

export default gql`
type Timeouts {
    queue: Int
    scheduling: Int
    nodestartup: Int
    driverstartup: Int
    sessiontermination: Int
    slotreclaiminterval: Int
}
`